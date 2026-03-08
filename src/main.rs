mod constants;
mod creator;
mod db;
mod describe;
mod extractor;
mod output;
mod query;
mod renamer;
mod renderer;
mod resolver;
mod scanner;
mod template;
mod verifier;

use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use std::env;
use std::path::PathBuf;

use crate::db::Database;

fn open_db(db_path: &std::path::Path) -> Result<Database, Box<dyn std::error::Error>> {
    Database::open_existing(db_path)
}

fn create_db(db_path: &std::path::Path) -> Result<Database, Box<dyn std::error::Error>> {
    Database::new(db_path)
}

const ENV_BASE_DIR: &str = "MARKBASE_BASE_DIR";
const ENV_INDEX_LOG_LEVEL: &str = "MARKBASE_INDEX_LOG_LEVEL";
const ENV_COMPUTE_BACKLINKS: &str = "MARKBASE_COMPUTE_BACKLINKS";

const VERSION: &str = env!("MARKBASE_VERSION");

#[derive(Clone, Copy, ValueEnum, Debug, PartialEq, Eq)]
enum OutputFormat {
    Table,
    List,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "table" => Ok(OutputFormat::Table),
            "list" => Ok(OutputFormat::List),
            _ => Err(format!("Invalid output format: {}", s)),
        }
    }
}

#[derive(Clone, Copy, ValueEnum, Debug, PartialEq, Eq)]
enum IndexLogLevel {
    Off,
    Summary,
    Verbose,
}

#[derive(Parser)]
#[command(name = "markbase")]
#[command(version = VERSION)]
#[command(about = "Markdown database CLI with automatic indexing", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(
        long = "base-dir",
        env = ENV_BASE_DIR,
        global = true,
        help_heading = "Environment Variables",
        help = "Directory to index (default: .)"
    )]
    base_dir: Option<PathBuf>,

    #[arg(
        long = "index-log-level",
        env = ENV_INDEX_LOG_LEVEL,
        global = true,
        value_enum,
        default_value_t = IndexLogLevel::Off,
        help = "Automatic indexing output: off, summary, or verbose"
    )]
    index_log_level: IndexLogLevel,

    #[arg(
        long = "compute-backlinks",
        env = ENV_COMPUTE_BACKLINKS,
        global = true,
        action = ArgAction::SetTrue,
        help = "Compute backlinks during automatic indexing"
    )]
    compute_backlinks: bool,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Query indexed notes")]
    Query {
        #[arg(value_name = "SQL")]
        sql: Option<String>,

        #[arg(short = 'o')]
        format: Option<OutputFormat>,

        #[arg(long = "abs-path")]
        abs_path: bool,

        #[arg(long = "dry-run", help = "Show translated SQL without executing")]
        dry_run: bool,
    },
    #[command(about = "Manage notes")]
    Note {
        #[command(subcommand)]
        command: NoteCommands,
    },
    #[command(about = "Manage templates")]
    Template {
        #[command(subcommand)]
        command: TemplateCommands,
    },
}

#[derive(Subcommand)]
enum NoteCommands {
    #[command(about = "Create a new markdown note with optional template")]
    New {
        name: String,

        #[arg(short, long)]
        template: Option<String>,
    },
    #[command(about = "Rename a note and update all links to it")]
    Rename { old_name: String, new_name: String },
    #[command(about = "Resolve one or more entity names to notes")]
    Resolve {
        #[arg(required = true, num_args = 1.., help = "One or more note or alias names")]
        names: Vec<String>,
    },
    #[command(about = "Verify a note against its template schema")]
    Verify {
        #[arg(help = "Note name (without .md extension)")]
        name: String,
    },
    #[command(about = "Render a note to stdout, expanding .base embeds")]
    Render {
        #[arg(help = "Note name (without .md extension)")]
        name: String,

        #[arg(short = 'o', help = "Output format: table (default) or list")]
        format: Option<OutputFormat>,

        #[arg(long = "dry-run", help = "Show SQL instead of executing queries")]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum TemplateCommands {
    #[command(about = "List all available templates")]
    List {
        #[arg(short = 'o', help = "Output format (default: table)")]
        format: Option<OutputFormat>,
    },
    #[command(about = "Show template content")]
    Describe {
        #[arg(help = "Template name (without .md extension)")]
        name: String,
    },
}

fn get_database_path(cli_base_dir: Option<PathBuf>) -> Result<PathBuf, String> {
    let base = get_base_dir_with_cli(cli_base_dir);
    let absolute = base
        .canonicalize()
        .map_err(|e| format!("Failed to resolve base-dir '{}': {}", base.display(), e))?;
    Ok(absolute.join(".markbase/markbase.duckdb"))
}

fn get_base_dir_with_cli(cli_base_dir: Option<PathBuf>) -> PathBuf {
    cli_base_dir
        .or_else(|| env::var(ENV_BASE_DIR).ok().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn get_base_dir_absolute_with_cli(cli_base_dir: Option<PathBuf>) -> Result<PathBuf, String> {
    let base = get_base_dir_with_cli(cli_base_dir);
    base.canonicalize()
        .map_err(|e| format!("Failed to resolve base-dir '{}': {}", base.display(), e))
}

fn output_format_name(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Table => "table",
        OutputFormat::List => "list",
    }
}

fn to_render_format(format: OutputFormat) -> renderer::RenderFormat {
    match format {
        OutputFormat::Table => renderer::RenderFormat::Table,
        OutputFormat::List => renderer::RenderFormat::List,
    }
}

fn check_db_exists(
    db_path: &std::path::Path,
    base_dir: &std::path::Path,
) -> Result<(), std::io::Error> {
    if db_path.exists() {
        return Ok(());
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!(
            "Database '.markbase/markbase.duckdb' not found at {}. Run a DB-backed command without '--dry-run' first.",
            base_dir.display()
        ),
    ))
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn ensure_index_ready(
    base_dir: &std::path::Path,
    db_path: &std::path::Path,
    compute_backlinks: bool,
) -> Result<(Database, scanner::IndexStats), Box<dyn std::error::Error>> {
    let db = create_db(db_path)?;
    let stats = scanner::index_directory_with_options(
        base_dir,
        &db,
        false,
        scanner::IndexOptions { compute_backlinks },
    )?;
    Ok((db, stats))
}

fn emit_index_output(stats: &scanner::IndexStats, log_level: IndexLogLevel) {
    if log_level == IndexLogLevel::Off {
        return;
    }

    if log_level == IndexLogLevel::Verbose {
        print_index_details(stats);
    }

    let time_str = format!(
        "{}.{}s",
        stats.duration_ms / 1000,
        (stats.duration_ms % 1000) / 100
    );
    eprintln!(
        "Indexed: {} new, {} updated, {} deleted, {} errors, {} warnings — {} total notes{}",
        stats.new,
        stats.updated,
        stats.deleted,
        stats.errors,
        stats.warning_count(),
        stats.total,
        if stats.duration_ms > 0 {
            format!("  [{}]", time_str)
        } else {
            String::new()
        }
    );
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let cli_base_dir = cli.base_dir.clone();
    let index_log_level = cli.index_log_level;
    let compute_backlinks = cli.compute_backlinks;

    let db_path = get_database_path(cli_base_dir.clone())?;
    let base_dir = get_base_dir_absolute_with_cli(cli_base_dir.clone())?;

    match cli.command {
        Commands::Query {
            sql,
            format,
            abs_path,
            dry_run,
        } => {
            let effective_format = format.unwrap_or(OutputFormat::Table);
            let format_str = output_format_name(effective_format);

            if dry_run {
                let translated =
                    query::translate_query(sql.as_deref()).map_err(|e| e.to_string())?;
                println!("{}", translated);
                return Ok(());
            }

            let (db, stats) = ensure_index_ready(&base_dir, &db_path, compute_backlinks)?;
            emit_index_output(&stats, index_log_level);
            let (field_names, results) =
                query::execute_query(&db, sql.as_deref()).map_err(|e| e.to_string())?;

            let abs_base_dir = if abs_path {
                Some(base_dir.as_path())
            } else {
                None
            };
            query::output_results(&results, format_str, &field_names, abs_base_dir, abs_path)?;
        }
        Commands::Note { command } => match command {
            NoteCommands::New { name, template } => {
                let created = creator::create_note(&base_dir, &name, template.as_deref())?;
                if template.is_some() {
                    println!("path: {}", created.path.display());
                    println!("content: {}", created.content);
                } else {
                    println!("Created: {}", created.path.display());
                }
            }
            NoteCommands::Rename { old_name, new_name } => {
                let result = renamer::rename_note(&base_dir, &old_name, &new_name)?;
                println!("Renamed: {} → {}", result.old_path, result.new_path);
                if !result.updated_files.is_empty() {
                    println!("Updated links in {} file(s):", result.updated_files.len());
                    for file in &result.updated_files {
                        println!("    ~ {}", file);
                    }
                }

                let (_db, stats) = ensure_index_ready(&base_dir, &db_path, compute_backlinks)?;
                emit_index_output(&stats, index_log_level);
            }
            NoteCommands::Resolve { names } => {
                let (db, stats) = ensure_index_ready(&base_dir, &db_path, compute_backlinks)?;
                emit_index_output(&stats, index_log_level);
                let results = resolver::resolve_names(&db, &names)?;
                println!("{}", serde_json::to_string_pretty(&results)?);
            }
            NoteCommands::Verify { name } => {
                let (db, stats) = ensure_index_ready(&base_dir, &db_path, compute_backlinks)?;
                emit_index_output(&stats, index_log_level);
                let result = verifier::verify_note(&base_dir, &db, &name)?;

                let template_list = result.template_names.join(", ");

                if result.issues.is_empty() {
                    println!(
                        "✓ note '{}' passed all checks against: {}.",
                        name, template_list
                    );
                    return Ok(());
                }

                eprintln!(
                    "Verifying note '{}' against template(s): {}\n",
                    name, template_list
                );
                for issue in &result.issues {
                    let prefix = match issue.level {
                        verifier::IssueLevel::Error => "[ERROR]",
                        verifier::IssueLevel::Warn => "[WARN]",
                        verifier::IssueLevel::Info => "[INFO]",
                    };
                    eprintln!("  {} {}", prefix, issue.message);
                    if let Some(ref def) = issue.field_definition {
                        eprintln!("  → Definition: {}", def);
                    }
                }
                eprintln!();

                if result.has_errors() {
                    eprintln!(
                        "Verification failed: {} error(s), {} warning(s).",
                        result.error_count(),
                        result.warn_count()
                    );
                    return Err(format!(
                        "note '{}' failed verification with {} error(s)",
                        name,
                        result.error_count()
                    )
                    .into());
                }

                eprintln!(
                    "Verification completed with issues: 0 error(s), {} warning(s).",
                    result.warn_count()
                );
            }
            NoteCommands::Render {
                name,
                format,
                dry_run,
            } => {
                let db = if dry_run {
                    check_db_exists(&db_path, &base_dir)?;
                    open_db(&db_path)?
                } else {
                    let (db, stats) = ensure_index_ready(&base_dir, &db_path, compute_backlinks)?;
                    emit_index_output(&stats, index_log_level);
                    db
                };

                let render_format = to_render_format(format.unwrap_or(OutputFormat::Table));
                let opts = renderer::RenderOptions {
                    format: render_format,
                    dry_run,
                };

                if let Err(e) = renderer::render_note(&base_dir, &db, &name, &opts) {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
        },
        Commands::Template { command } => match command {
            TemplateCommands::List { format } => {
                let sql = "SELECT file.name, _schema.description, file.path FROM notes WHERE file.folder=='templates'";
                let (db, stats) = ensure_index_ready(&base_dir, &db_path, compute_backlinks)?;
                emit_index_output(&stats, index_log_level);
                let (field_names, results) =
                    query::execute_query(&db, Some(sql)).map_err(|e| e.to_string())?;

                let effective_format = format.unwrap_or(OutputFormat::Table);
                let format_str = output_format_name(effective_format);
                query::output_results(&results, format_str, &field_names, None, false)?;
            }
            TemplateCommands::Describe { name } => {
                let content = describe::describe_template(&base_dir, &name)?;
                println!("{}", content);
            }
        },
    }

    Ok(())
}

fn print_index_details(stats: &scanner::IndexStats) {
    if !stats.new_files.is_empty() {
        for path in &stats.new_files {
            let rel = stats.relative_path(path);
            eprintln!("    + {}", rel);
        }
    }
    if !stats.updated_files.is_empty() {
        for path in &stats.updated_files {
            let rel = stats.relative_path(path);
            eprintln!("    ~ {}", rel);
        }
    }
    if !stats.deleted_files.is_empty() {
        for path in &stats.deleted_files {
            let rel = stats.relative_path(path);
            eprintln!("    - {}", rel);
        }
    }

    for diagnostic in &stats.diagnostics {
        let prefix = match diagnostic.level {
            scanner::IndexDiagnosticLevel::Warn => "⚠",
            scanner::IndexDiagnosticLevel::Error => "✗",
        };
        if let Some(path) = &diagnostic.path {
            eprintln!("  {} {} — {}", prefix, path, diagnostic.message);
        } else {
            eprintln!("  {} {}", prefix, diagnostic.message);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_command_removed() {
        let result = Cli::try_parse_from(["markbase", "index"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_global_index_log_level_option() {
        let cli = Cli::parse_from([
            "markbase",
            "--index-log-level",
            "verbose",
            "query",
            "name == 'test'",
        ]);
        assert_eq!(cli.index_log_level, IndexLogLevel::Verbose);
    }

    #[test]
    fn test_global_compute_backlinks_option() {
        let cli = Cli::parse_from(["markbase", "--compute-backlinks", "query", "name == 'test'"]);
        assert!(cli.compute_backlinks);
    }

    #[test]
    fn test_query_with_sql() {
        let cli = Cli::parse_from(["markbase", "query", "name == 'test'"]);
        if let Commands::Query { sql, .. } = cli.command {
            assert_eq!(sql, Some("name == 'test'".to_string()));
        } else {
            panic!("Expected Query command");
        }
    }

    #[test]
    fn test_query_without_sql() {
        let cli = Cli::parse_from(["markbase", "query"]);
        if let Commands::Query { sql, .. } = cli.command {
            assert_eq!(sql, None);
        } else {
            panic!("Expected Query command");
        }
    }

    #[test]
    fn test_query_output_format_option() {
        let cli = Cli::parse_from(["markbase", "query", "name == 'test'", "-o", "list"]);
        if let Commands::Query { format, .. } = cli.command {
            assert_eq!(format, Some(OutputFormat::List));
        } else {
            panic!("Expected Query command");
        }
    }

    #[test]
    fn test_abs_path_option_default() {
        let cli = Cli::parse_from(["markbase", "query", "name == 'test'"]);
        if let Commands::Query { abs_path, .. } = cli.command {
            assert!(!abs_path);
        } else {
            panic!("Expected Query command");
        }
    }

    #[test]
    fn test_abs_path_option_enabled() {
        let cli = Cli::parse_from(["markbase", "query", "name == 'test'", "--abs-path"]);
        if let Commands::Query { abs_path, .. } = cli.command {
            assert!(abs_path);
        } else {
            panic!("Expected Query command");
        }
    }

    #[test]
    fn test_dry_run_option() {
        let cli = Cli::parse_from(["markbase", "query", "name == 'test'", "--dry-run"]);
        if let Commands::Query { dry_run, .. } = cli.command {
            assert!(dry_run);
        } else {
            panic!("Expected Query command");
        }
    }

    #[test]
    fn test_note_new_command_basic() {
        let cli = Cli::parse_from(["markbase", "note", "new", "my-note"]);
        if let Commands::Note { command } = cli.command {
            match command {
                NoteCommands::New { name, template } => {
                    assert_eq!(name, "my-note");
                    assert_eq!(template, None);
                }
                _ => panic!("Expected New command"),
            }
        } else {
            panic!("Expected Note command");
        }
    }

    #[test]
    fn test_note_new_command_with_template() {
        let cli = Cli::parse_from(["markbase", "note", "new", "my-note", "--template", "daily"]);
        if let Commands::Note { command } = cli.command {
            match command {
                NoteCommands::New { name, template } => {
                    assert_eq!(name, "my-note");
                    assert_eq!(template, Some("daily".to_string()));
                }
                _ => panic!("Expected New command"),
            }
        } else {
            panic!("Expected Note command");
        }
    }

    #[test]
    fn test_note_rename_command() {
        let cli = Cli::parse_from(["markbase", "note", "rename", "old-name", "new-name"]);
        if let Commands::Note { command } = cli.command {
            match command {
                NoteCommands::Rename { old_name, new_name } => {
                    assert_eq!(old_name, "old-name");
                    assert_eq!(new_name, "new-name");
                }
                _ => panic!("Expected Rename command"),
            }
        } else {
            panic!("Expected Note command");
        }
    }

    #[test]
    fn test_note_render_output_format_option() {
        let cli = Cli::parse_from(["markbase", "note", "render", "demo", "-o", "list"]);
        if let Commands::Note { command } = cli.command {
            match command {
                NoteCommands::Render { format, .. } => {
                    assert_eq!(format, Some(OutputFormat::List));
                }
                _ => panic!("Expected Render command"),
            }
        } else {
            panic!("Expected Note command");
        }
    }

    #[test]
    fn test_note_render_rejects_json_option() {
        let result = Cli::try_parse_from(["markbase", "note", "render", "demo", "-o", "json"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_template_list_command() {
        let cli = Cli::parse_from(["markbase", "template", "list"]);
        if let Commands::Template { command } = cli.command {
            match command {
                TemplateCommands::List { format } => assert_eq!(format, None),
                TemplateCommands::Describe { .. } => panic!("Expected List command, got Describe"),
            }
        } else {
            panic!("Expected Template command");
        }
    }

    #[test]
    fn test_template_list_with_output_format() {
        let cli = Cli::parse_from(["markbase", "template", "list", "-o", "list"]);
        if let Commands::Template { command } = cli.command {
            match command {
                TemplateCommands::List { format } => assert_eq!(format, Some(OutputFormat::List)),
                TemplateCommands::Describe { .. } => panic!("Expected List command, got Describe"),
            }
        } else {
            panic!("Expected Template command");
        }
    }

    #[test]
    fn test_template_describe_command() {
        let cli = Cli::parse_from(["markbase", "template", "describe", "daily"]);
        if let Commands::Template { command } = cli.command {
            match command {
                TemplateCommands::List { .. } => panic!("Expected Describe command, got List"),
                TemplateCommands::Describe { name } => assert_eq!(name, "daily"),
            }
        } else {
            panic!("Expected Template command");
        }
    }
}

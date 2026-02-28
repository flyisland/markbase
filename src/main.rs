mod constants;
mod creator;
mod db;
mod describe;
mod extractor;
mod query;
mod scanner;

use clap::{Parser, Subcommand, ValueEnum};
use std::env;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::db::Database;

const ENV_BASE_DIR: &str = "MDB_BASE_DIR";
const ENV_OUTPUT: &str = "MDB_OUTPUT";

const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("VERGEN_GIT_SHA"),
    " ",
    env!("VERGEN_GIT_COMMIT_TIMESTAMP"),
    ")"
);

#[derive(Clone, ValueEnum, Debug, PartialEq)]
enum OutputFormat {
    Table,
    Json,
    List,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "table" => Ok(OutputFormat::Table),
            "json" => Ok(OutputFormat::Json),
            "list" => Ok(OutputFormat::List),
            _ => Err(format!("Invalid output format: {}", s)),
        }
    }
}

#[derive(Parser)]
#[command(name = "mdb")]
#[command(version = VERSION)]
#[command(about = "Markdown database CLI - index and query markdown files", long_about = None)]
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
        long = "output-format",
        global = true,
        env = ENV_OUTPUT,
        help_heading = "Output",
        help = "Output format: table, json, list"
    )]
    output_format: Option<OutputFormat>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Scan and index markdown files into database")]
    Index {
        #[arg(short, long)]
        force: bool,

        #[arg(short, long)]
        verbose: bool,
    },
    #[command(about = "Query indexed files with SQL-like expressions")]
    Query {
        #[arg(value_name = "QUERY")]
        query: String,

        #[arg(short = 'o')]
        format: Option<OutputFormat>,

        #[arg(short = 'F', long = "output-fields", default_value = "path, mtime")]
        fields: String,

        #[arg(short, long, default_value_t = 1000)]
        limit: usize,
    },
    #[command(about = "Create a new markdown note with optional template")]
    New {
        name: String,

        #[arg(short, long)]
        template: Option<String>,
    },
    #[command(about = "Manage templates")]
    Template {
        #[command(subcommand)]
        command: TemplateCommands,
    },
}

#[derive(Subcommand)]
enum TemplateCommands {
    #[command(about = "List all available templates")]
    List {
        #[arg(
            short = 'F',
            long = "additional-fields",
            help = "Additional fields to display"
        )]
        fields: Option<String>,

        #[arg(short = 'o', help = "Output format (default: json)")]
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
        .map_err(|e| format!("Failed to resolve base-dir: {}", e))?;
    Ok(absolute.join(".mdb/mdb.duckdb"))
}

fn get_base_dir_with_cli(cli_base_dir: Option<PathBuf>) -> PathBuf {
    cli_base_dir
        .or_else(|| env::var(ENV_BASE_DIR).ok().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn get_base_dir_absolute_with_cli(cli_base_dir: Option<PathBuf>) -> Result<PathBuf, String> {
    let base = get_base_dir_with_cli(cli_base_dir);
    base.canonicalize()
        .map_err(|e| format!("Failed to resolve base-dir: {}", e))
}

fn get_output_format(cli_format: Option<OutputFormat>) -> OutputFormat {
    cli_format.unwrap_or_else(|| {
        env::var(ENV_OUTPUT)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(OutputFormat::Table)
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let db_path = get_database_path(cli.base_dir.clone())?;

    match cli.command {
        Commands::Index { force, verbose } => {
            let base = get_base_dir_absolute_with_cli(cli.base_dir.clone())?;
            let db = Mutex::new(Database::new(&db_path)?);
            let db = db.lock().unwrap();
            eprintln!("Indexing {}...", base.display());
            let stats = scanner::index_directory(&base, &db, force)?;

            if verbose {
                if !stats.new_files.is_empty() {
                    for path in &stats.new_files {
                        let rel = stats.relative_path(path);
                        println!("    + {}", rel);
                    }
                }
                if !stats.updated_files.is_empty() {
                    for path in &stats.updated_files {
                        let rel = stats.relative_path(path);
                        println!("    ~ {}", rel);
                    }
                }

                for (path, reason) in &stats.skipped {
                    if reason != "unchanged" {
                        eprintln!("  ⚠ Skipped: {} — {}", path, reason);
                    }
                }
            }

            let total = stats.new + stats.updated;
            let details = if stats.new > 0 || stats.updated > 0 || stats.errors > 0 {
                format!(
                    " ({} new, {} updated, {} errors)",
                    stats.new, stats.updated, stats.errors
                )
            } else {
                String::new()
            };
            let time_str = format!(
                "{}.{}s",
                stats.duration_ms / 1000,
                (stats.duration_ms % 1000) / 100
            );
            println!(
                "  ✓ {} files indexed{}{}",
                total,
                details,
                if stats.duration_ms > 0 {
                    format!("  [{}]", time_str)
                } else {
                    String::new()
                }
            );
        }
        Commands::Query {
            query,
            format,
            limit,
            fields,
        } => {
            let field_names: Vec<String> =
                fields.split(',').map(|s| s.trim().to_string()).collect();
            let effective_format = get_output_format(format.or(cli.output_format));
            let format_str = match effective_format {
                OutputFormat::Table => "table",
                OutputFormat::Json => "json",
                OutputFormat::List => "list",
            };
            let compiled = query::build_sql(&query, &fields).map_err(|e| e.to_string())?;
            let db = Mutex::new(Database::open_existing(&db_path)?);
            let db = db.lock().unwrap();
            let results = db
                .query(&compiled, &fields, limit)
                .map_err(|e| db::convert_duckdb_error(&e.to_string(), &query))?;
            query::output_results(&results, format_str, &field_names)?;
        }
        Commands::New { name, template } => {
            let base = get_base_dir_absolute_with_cli(cli.base_dir.clone())?;
            let created = creator::create_note(&base, &name, template.as_deref())?;
            if template.is_some() {
                println!("path: {}", created.path.display());
                println!("content: {}", created.content);
            } else {
                println!("Created: {}", created.path.display());
            }
        }
        Commands::Template { command } => match command {
            TemplateCommands::List { fields, format } => {
                let mut output_fields = vec![
                    "name".to_string(),
                    "_schema.description".to_string(),
                    "path".to_string(),
                ];

                if let Some(extra) = fields {
                    let user_fields: Vec<String> =
                        extra.split(',').map(|s| s.trim().to_string()).collect();
                    output_fields.extend(user_fields);
                }

                let fields_str = output_fields.join(", ");
                let query = "folder=='templates'".to_string();
                let compiled = query::build_sql(&query, &fields_str).map_err(|e| e.to_string())?;
                let db = Mutex::new(Database::open_existing(&db_path)?);
                let db = db.lock().unwrap();
                let results = db
                    .query(&compiled, &fields_str, 1000)
                    .map_err(|e| db::convert_duckdb_error(&e.to_string(), &query))?;

                let effective_format = format.or(cli.output_format).unwrap_or(OutputFormat::Json);
                let format_str = match effective_format {
                    OutputFormat::Table => "table",
                    OutputFormat::Json => "json",
                    OutputFormat::List => "list",
                };
                query::output_results(&results, format_str, &output_fields)?;
            }
            TemplateCommands::Describe { name } => {
                let base = get_base_dir_absolute_with_cli(cli.base_dir.clone())?;
                let content = describe::describe_template(&base, &name)?;
                println!("{}", content);
            }
        },
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_fields_value() {
        let cli = Cli::parse_from(["mdb", "query", "name == 'test'"]);
        if let Commands::Query { fields, .. } = cli.command {
            assert_eq!(fields, "path, mtime");
        } else {
            panic!("Expected Query command");
        }
    }

    #[test]
    fn test_all_fields_option() {
        let cli = Cli::parse_from(["mdb", "query", "file.name == 'test'", "-F", "*"]);
        if let Commands::Query { fields, .. } = cli.command {
            assert_eq!(fields, "*");
        } else {
            panic!("Expected Query command");
        }
    }

    #[test]
    fn test_specific_field_option() {
        let cli = Cli::parse_from([
            "mdb",
            "query",
            "file.name == 'test'",
            "--output-fields",
            "file.name",
        ]);
        if let Commands::Query { fields, .. } = cli.command {
            assert_eq!(fields, "file.name");
        } else {
            panic!("Expected Query command");
        }
    }

    #[test]
    fn test_output_format_option() {
        let cli = Cli::parse_from(["mdb", "query", "file.name == 'test'", "-o", "json"]);
        if let Commands::Query { format, .. } = cli.command {
            assert_eq!(format, Some(OutputFormat::Json));
        } else {
            panic!("Expected Query command");
        }
    }

    #[test]
    fn test_new_command_basic() {
        let cli = Cli::parse_from(["mdb", "new", "my-note"]);
        if let Commands::New { name, template } = cli.command {
            assert_eq!(name, "my-note");
            assert_eq!(template, None);
        } else {
            panic!("Expected New command");
        }
    }

    #[test]
    fn test_new_command_with_template() {
        let cli = Cli::parse_from(["mdb", "new", "my-note", "--template", "daily"]);
        if let Commands::New { name, template } = cli.command {
            assert_eq!(name, "my-note");
            assert_eq!(template, Some("daily".to_string()));
        } else {
            panic!("Expected New command");
        }
    }

    #[test]
    fn test_template_list_command() {
        let cli = Cli::parse_from(["mdb", "template", "list"]);
        if let Commands::Template { command } = cli.command {
            match command {
                TemplateCommands::List { fields, format } => {
                    assert_eq!(fields, None);
                    assert_eq!(format, None);
                }
                TemplateCommands::Describe { .. } => {
                    panic!("Expected List command, got Describe");
                }
            }
        } else {
            panic!("Expected Template command");
        }
    }

    #[test]
    fn test_template_list_with_fields() {
        let cli = Cli::parse_from(["mdb", "template", "list", "-F", "tags,type"]);
        if let Commands::Template { command } = cli.command {
            match command {
                TemplateCommands::List { fields, format } => {
                    assert_eq!(fields, Some("tags,type".to_string()));
                    assert_eq!(format, None);
                }
                TemplateCommands::Describe { .. } => {
                    panic!("Expected List command, got Describe");
                }
            }
        } else {
            panic!("Expected Template command");
        }
    }

    #[test]
    fn test_template_list_with_output_format() {
        let cli = Cli::parse_from(["mdb", "template", "list", "-o", "json"]);
        if let Commands::Template { command } = cli.command {
            match command {
                TemplateCommands::List { fields, format } => {
                    assert_eq!(fields, None);
                    assert_eq!(format, Some(OutputFormat::Json));
                }
                TemplateCommands::Describe { .. } => {
                    panic!("Expected List command, got Describe");
                }
            }
        } else {
            panic!("Expected Template command");
        }
    }

    #[test]
    fn test_template_describe_command() {
        let cli = Cli::parse_from(["mdb", "template", "describe", "daily"]);
        if let Commands::Template { command } = cli.command {
            match command {
                TemplateCommands::List { .. } => {
                    panic!("Expected Describe command, got List");
                }
                TemplateCommands::Describe { name } => {
                    assert_eq!(name, "daily");
                }
            }
        } else {
            panic!("Expected Template command");
        }
    }

    #[test]
    fn test_global_output_format() {
        let cli = Cli::parse_from(["mdb", "--output-format", "json", "query", "name == 'test'"]);
        assert_eq!(cli.output_format, Some(OutputFormat::Json));
    }

    #[test]
    fn test_query_format_overrides_global() {
        let cli = Cli::parse_from([
            "mdb",
            "--output-format",
            "json",
            "query",
            "name == 'test'",
            "-o",
            "list",
        ]);
        if let Commands::Query { format, .. } = cli.command {
            assert_eq!(format, Some(OutputFormat::List));
        } else {
            panic!("Expected Query command");
        }
    }

    #[test]
    fn test_template_list_format_overrides_global() {
        let cli = Cli::parse_from([
            "mdb",
            "--output-format",
            "table",
            "template",
            "list",
            "-o",
            "json",
        ]);
        if let Commands::Template { command } = cli.command {
            match command {
                TemplateCommands::List { format, .. } => {
                    assert_eq!(format, Some(OutputFormat::Json));
                }
                _ => panic!("Expected List command"),
            }
        } else {
            panic!("Expected Template command");
        }
    }
}

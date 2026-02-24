mod creator;
mod db;
mod extractor;
mod query;
mod scanner;
mod watcher;

use clap::{Parser, Subcommand, ValueEnum};
use std::env;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::db::Database;

const ENV_DATABASE: &str = "MDB_DATABASE";
const ENV_BASE_DIR: &str = "MDB_BASE_DIR";

#[derive(Clone, ValueEnum, Debug, PartialEq)]
enum OutputFormat {
    Table,
    Json,
    List,
}

#[derive(Parser)]
#[command(name = "mdb")]
#[command(version = "0.1.0")]
#[command(about = "Markdown database CLI - index and query markdown files", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(
        short,
        long,
        env = ENV_DATABASE,
        global = true,
        help_heading = "Environment Variables",
        help = "Path to DuckDB database (default: .mdb/mdb.duckdb)"
    )]
    database: Option<PathBuf>,

    #[arg(
        short = 'b',
        long = "base-dir",
        env = ENV_BASE_DIR,
        global = true,
        help_heading = "Environment Variables",
        help = "Directory to index (default: .)"
    )]
    base_dir: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Scan and index markdown files into database")]
    Index {
        #[arg(short, long)]
        force: bool,

        #[arg(short, long)]
        verbose: bool,

        #[arg(
            short,
            long,
            help = "Watch for file changes and re-index automatically"
        )]
        watch: bool,
    },
    #[command(about = "Query indexed files with SQL-like expressions")]
    Query {
        #[arg(value_name = "QUERY")]
        query: String,

        #[arg(short = 'o', long = "output-format", default_value = "table")]
        format: OutputFormat,

        #[arg(
            short = 'f',
            long = "output-fields",
            default_value = "file.path, file.mtime"
        )]
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
}

fn get_database_path() -> PathBuf {
    env::var(ENV_DATABASE)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(".mdb/mdb.duckdb"))
}

fn get_base_dir() -> PathBuf {
    env::var(ENV_BASE_DIR)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn run_watch_mode(
    base_dir: &PathBuf,
    db_path: &PathBuf,
    force: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Initial indexing...");
    let db = Database::new(db_path)?;
    scanner::index_directory(base_dir, &db, force, verbose, None)?;
    drop(db);

    println!(
        "\nWatching for changes in {} (Ctrl+C to stop)\n",
        base_dir.display()
    );

    let watcher = watcher::FileWatcher::new(base_dir)?;

    loop {
        match watcher.wait_for_changes_with_kind() {
            Ok(changes) => {
                if changes.is_empty() {
                    continue;
                }

                let (creates_modifies, removes): (Vec<_>, Vec<_>) =
                    changes.into_iter().partition(|(_, kind)| {
                        matches!(kind, notify_debouncer_mini::DebouncedEventKind::Any)
                    });

                let db = Database::new(db_path)?;

                for (path, _) in removes {
                    let path_str = path.canonicalize()?.to_string_lossy().to_string();
                    if let Some(name) = path.file_stem() {
                        let name_str = name.to_string_lossy().to_string();
                        if verbose {
                            println!("File removed: {}", path_str);
                        }
                        db.delete_document(&path_str)?;
                        scanner::update_backlinks_after_delete(&db, &name_str)?;
                    }
                }

                if !creates_modifies.is_empty() {
                    let paths: Vec<PathBuf> =
                        creates_modifies.into_iter().map(|(path, _)| path).collect();

                    if verbose {
                        for path in &paths {
                            println!("File changed: {}", path.display());
                        }
                    }

                    scanner::index_directory(base_dir, &db, false, verbose, Some(paths))?;
                }

                drop(db);

                println!(
                    "\nWatching for changes in {} (Ctrl+C to stop)\n",
                    base_dir.display()
                );
            }
            Err(e) => {
                eprintln!("Watch error: {}", e);
                break;
            }
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let db_path = cli.database.unwrap_or_else(get_database_path);

    let db = Mutex::new(Database::new(&db_path)?);

    match cli.command {
        Commands::Index {
            force,
            verbose,
            watch,
        } => {
            let base = cli.base_dir.unwrap_or_else(get_base_dir);
            if watch {
                run_watch_mode(&base, &db_path, force, verbose)?;
            } else {
                let db = db.lock().unwrap();
                scanner::index_directory(&base, &db, force, verbose, None)?;
            }
        }
        Commands::Query {
            query,
            format,
            limit,
            fields,
        } => {
            let field_names: Vec<String> =
                fields.split(',').map(|s| s.trim().to_string()).collect();
            let format_str = match format {
                OutputFormat::Table => "table",
                OutputFormat::Json => "json",
                OutputFormat::List => "list",
            };
            let compiled = query::build_sql(&query, &fields).map_err(|e| e.to_string())?;
            let db = db.lock().unwrap();
            let results = db.query(&compiled, &fields, limit)?;
            query::output_results(&results, format_str, &field_names)?;
        }
        Commands::New { name, template } => {
            let base = cli.base_dir.unwrap_or_else(get_base_dir);
            let created_path = creator::create_note(&base, &name, template.as_deref())?;
            println!("Created: {}", created_path.display());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_fields_value() {
        let cli = Cli::parse_from(["mdb", "query", "-q", "file.name == 'test'"]);
        if let Commands::Query { fields, .. } = cli.command {
            assert_eq!(fields, "file.path, file.mtime");
        } else {
            panic!("Expected Query command");
        }
    }

    #[test]
    fn test_all_fields_option() {
        let cli = Cli::parse_from(["mdb", "query", "-q", "file.name == 'test'", "-f", "*"]);
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
            "-q",
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
        let cli = Cli::parse_from(["mdb", "query", "-q", "file.name == 'test'", "-o", "json"]);
        if let Commands::Query { format, .. } = cli.command {
            assert_eq!(format, OutputFormat::Json);
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
    fn test_index_watch_flag() {
        let cli = Cli::parse_from(["mdb", "index", "--watch"]);
        if let Commands::Index {
            force,
            verbose,
            watch,
        } = cli.command
        {
            assert_eq!(force, false);
            assert_eq!(verbose, false);
            assert_eq!(watch, true);
        } else {
            panic!("Expected Index command");
        }
    }

    #[test]
    fn test_index_watch_with_options() {
        let cli = Cli::parse_from(["mdb", "index", "--force", "--verbose", "--watch"]);
        if let Commands::Index {
            force,
            verbose,
            watch,
        } = cli.command
        {
            assert_eq!(force, true);
            assert_eq!(verbose, true);
            assert_eq!(watch, true);
        } else {
            panic!("Expected Index command");
        }
    }
}

mod constants;
mod creator;
mod db;
mod extractor;
mod query;
mod scanner;

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
            default_value = "path, mtime"
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
        #[arg(short, long, help = "Additional fields to display")]
        fields: Option<String>,
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let db_path = cli.database.unwrap_or_else(get_database_path);

    let db = Mutex::new(Database::new(&db_path)?);

    match cli.command {
        Commands::Index { force, verbose } => {
            let base = cli.base_dir.unwrap_or_else(get_base_dir);
            let base_abs = base.canonicalize().map_err(|e| format!("Failed to resolve base-dir: {}", e))?;
            let db = db.lock().unwrap();
            eprintln!("Indexing {}...", base_abs.display());
            let stats = scanner::index_directory(&base, &base_abs, &db, force, None)?;
            
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
                format!(" ({} new, {} updated, {} errors)", stats.new, stats.updated, stats.errors)
            } else {
                String::new()
            };
            let time_str = format!("{}.{}s", stats.duration_ms / 1000, (stats.duration_ms % 1000) / 100);
            println!("  ✓ {} files indexed{}{}", total, details, if stats.duration_ms > 0 { format!("  [{}]", time_str) } else { String::new() });
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
            if compiled.contains("_arg_should_not_be_quoted") {
                return Err("Error: property name in function should not be quoted. Use function(property_name, ...) instead of function('property_name', ...)".into());
            }
            let db = db.lock().unwrap();
            let results = db.query(&compiled, &fields, limit)?;
            query::output_results(&results, format_str, &field_names)?;
        }
        Commands::New { name, template } => {
            let base = cli.base_dir.unwrap_or_else(get_base_dir);
            let created_path = creator::create_note(&base, &name, template.as_deref())?;
            println!("Created: {}", created_path.display());
        }
        Commands::Template { command } => match command {
            TemplateCommands::List { fields } => {
                let base = cli.base_dir.unwrap_or_else(get_base_dir);
                let base_canonical = base
                    .canonicalize()
                    .map_err(|e| format!("Failed to resolve base-dir: {}", e))?;
                let pattern = format!("{}/templates/%%", base_canonical.display());

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
                let query = format!("path=~\"{}\"", pattern);
                let compiled = query::build_sql(&query, &fields_str).map_err(|e| e.to_string())?;
                if compiled.contains("_arg_should_not_be_quoted") {
                    return Err("Error: property name in function should not be quoted. Use function(property_name, ...) instead of function('property_name', ...)".into());
                }
                let db = db.lock().unwrap();
                let results = db.query(&compiled, &fields_str, 1000)?;
                query::output_results(&results, "list", &output_fields)?;
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
        let cli = Cli::parse_from(["mdb", "query", "file.name == 'test'", "-f", "*"]);
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
    fn test_template_list_command() {
        let cli = Cli::parse_from(["mdb", "template", "list"]);
        if let Commands::Template { command } = cli.command {
            let TemplateCommands::List { fields } = command;
            assert_eq!(fields, None);
        } else {
            panic!("Expected Template command");
        }
    }

    #[test]
    fn test_template_list_with_fields() {
        let cli = Cli::parse_from(["mdb", "template", "list", "-f", "tags,type"]);
        if let Commands::Template { command } = cli.command {
            let TemplateCommands::List { fields } = command;
            assert_eq!(fields, Some("tags,type".to_string()));
        } else {
            panic!("Expected Template command");
        }
    }
}

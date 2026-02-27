# Agent Specification: Markdown Base CLI

## 1. Project Overview

The goal is to build a high-performance Command Line Interface (CLI) tool designed to scan, parse, and index Markdown files into a **DuckDB** database for instantaneous metadata searching with Obsidian compatibility in mind.

## 2. Technical Stack

See [README.md](./README.md#tech-stack) for the complete tech stack details.

## 3. Data Schema

The DuckDB local file (`.mdb/mdb.duckdb`) utilizes the following schema to support Markdown structures:

| Property   | Type       | Description                                                   |
| ---------- | ---------- | ------------------------------------------------------------- |
| path       | TEXT       | Primary key - full file path                                  |
| folder     | TEXT       | Directory path                                                |
| name       | TEXT       | File name (without extension)                                 |
| ext        | TEXT       | File extension                                                |
| size       | INTEGER    | File size in bytes                                            |
| ctime      | TIMESTAMPTZ| Created time                                                   |
| mtime      | TIMESTAMPTZ| Modified time                                                 |
| content    | TEXT       | Full file content (including frontmatter)                    |
| tags       | VARCHAR[]  | Array of tags                                                 |
| links      | VARCHAR[]  | Array of wiki-links                                           |
| backlinks  | VARCHAR[]  | Array of backlink files                                        |
| embeds     | VARCHAR[]  | Array of embeds                                               |
| properties | JSON       | Frontmatter properties                                        |

**Note**: Array types stored as native VARCHAR[] arrays, properties as JSON.

### 3.1 Indexes
```sql
CREATE INDEX IF NOT EXISTS idx_mtime ON documents(mtime);
CREATE INDEX IF NOT EXISTS idx_folder ON documents(folder);
CREATE INDEX IF NOT EXISTS idx_name ON documents(name);
```

## 4. Operational Requirements

### Command: `index`
- **Behavior**: Scans a target directory for `.md` files recursively.
- **Concurrency**: Sequential processing with WalkDir iterator.
- **Logic**: 
    - Perform incremental updates by comparing `mtime`.
    - Extract YAML Frontmatter using `gray_matter`.
    - Parse wiki-links `[[link]]`, embeds `![[embed]]`, and tags `#tag` using regex.
    - Calculate backlinks (reverse link lookup) post-indexing.
    - Insert documents using parameterized queries with `duckdb::params!`.
- **Options**:
    - `-b, --base-dir <path>` - Target directory (default: current directory `.`)
    - `-f, --force` - Force re-index all files (ignore mtime)
    - `-v, --verbose` - Show detailed output

### Command: `query`
- **Behavior**: Query indexed files with SQL-like expressions.
- **Functionality**: 
    - Parse SQL-like query expressions with field references, operators, and logical combinations
    - Field resolution: reserved fields (path, folder, name, ext, size, ctime, mtime, content, tags, links, backlinks, embeds) are checked first, then frontmatter properties
    - Support nested JSON paths (e.g., `a.b.c` → `json_extract_string(properties, '$.\"a\".\"b\".\"c\"')`)
    - Support comparison operators: `==`, `!=`, `>`, `<`, `>=`, `<=`, `=~` (pattern match), and `=` (single equals)
    - Support logical operators: `and`, `or` with proper precedence (and has higher precedence than or)
    - Support `has()` function for array containment checks
    - Compile queries to DuckDB SQL for execution
    - Timestamps displayed in human-readable format (YYYY-MM-DD HH:MM:SS)
    - Warning when frontmatter field conflicts with reserved field (except `tags`)
- **Options**:
    - `<query>` - Query expression (positional argument, required)
    - `-o, --output-format <type>` - Output: table, json, list (default: table)
    - `-l, --limit <n>` - Max results (default: 1000)
    - `-f, --output-fields <fields>` - Fields to select (default: path, mtime)

### Command: `new`
- **Behavior**: Create a new markdown note with optional template.
- **Options**:
    - `<name>` - Note name (positional argument, required)
    - `-t, --template <name>` - Template name to use (optional)

## 5. Implementation Details

### 5.1 File Discovery (scanner.rs)
Uses `walkdir` to recursively find `.md` files:
```rust
for entry in WalkDir::new(dir)
    .follow_links(true)
    .into_iter()
    .filter_map(|e| e.ok())
{
    let path = entry.path();
    if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
        // Process markdown file
    }
}
```

### 5.2 Content Parsing (extractor.rs)
Regex patterns for Obsidian-style markdown:
```rust
static WIKILINK_REGEX: &str = r"\[\[([^\]]+)\]\]";
static EMBED_REGEX: &str = r"!\[\[([^\]]+)\]\]";
static TAG_REGEX: &str = r"#[\w\-/]+";
```

Frontmatter parsing with gray_matter:
```rust
fn parse_frontmatter(content: &str) -> (Value, String) {
    let matter = Matter::<YAML>::new();
    match matter.parse::<Value>(content) {
        Ok(result) => {
            let frontmatter = result.data.map(|v| serde_json::to_value(v).unwrap_or(Value::Null)).unwrap_or(Value::Null);
            (frontmatter, result.content)
        }
        Err(_) => (Value::Null, content.to_string()),
    }
}
```

### 5.3 Database Operations (db.rs)
Using `duckdb` crate with Rust API:
```rust
pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let db = Database { conn };
        db.init_schema()?;
        Ok(db)
    }
    
    pub fn upsert_document(&self, doc: &Document) -> Result<(), Box<dyn std::error::Error>> {
        self.conn.execute(
            "INSERT OR REPLACE INTO documents ...",
            params![...],
        )?;
        Ok(())
    }
}
```

**Row Access Pattern**:
```rust
let mut stmt = self.conn.prepare("SELECT * FROM documents")?;
let mut rows = stmt.query([])?;

while let Some(row) = rows.next()? {
    let val: duckdb::types::Value = row.get(i)?;
    // Process value
}
```

### 5.4 Query System (query/)
Located in `src/query/`:

- **tokenizer.rs**: Tokenizes query expressions into tokens (Field, Operator, StringLiteral, NumberLiteral, LParen, RParen, Comma, Function, And, Or, EOF)
  ```rust
  pub enum Token {
      Field(String),
      Operator(String),
      StringLiteral(String),
      NumberLiteral(String),
      LParen, RParen, Comma,
      Function(String),
      And, Or, EOF,
  }
  ```

- **parser.rs**: Recursive descent parser building AST from tokens
  ```rust
  pub enum AstNode {
      Binary { left: Box<AstNode>, op: String, right: Box<AstNode> },
      Field(String),
      StringLiteral(String),
      NumberLiteral(String),
      FunctionCall { name: String, args: Vec<AstNode> },
      Grouping(Box<AstNode>),
  }
  ```

- **compiler.rs**: Compiles AST to DuckDB SQL with field resolution
  ```rust
  pub fn resolve_field(field: &str) -> String {
      // Reserved fields first, then JSON path for nested/frontmatter properties
      if RESERVED_FIELDS.contains(&field) {
          return field.to_string();
      }
      // Handle nested JSON paths like "a.b.c"
      format!("json_extract_string(properties, '$.\"{}\"')", field.replace('.', "\".\""))
  }
  ```

- **mod.rs**: Output formatting (table, json, list)

### 5.5 CLI Entry Point (main.rs)
Using clap derive macros:
```rust
#[derive(Parser)]
#[command(name = "mdb")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(short, long)]
    database: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    Index { force, verbose },
    Query { query, format, limit, fields },
    New { name, template },
}
```

## 6. Performance Goals
- **Indexing Speed**: < 5 seconds for 10,000 files (cold start).
- **Search Latency**: < 50ms for complex queries on 10,000 rows.
- **Query Latency**: < 100ms for complex query expressions.
- **Binary Size**: Optimized release builds with `strip = true` in Cargo.toml.

## 7. Constraints & Safety
- **Single Writer**: Only one `index` process can run at a time (DuckDB constraint).
- **Graceful Shutdown**: Database connection closed when `Database` struct is dropped.
- **Error Handling**: Comprehensive use of `Result` type with `?` operator for propagation.
- **Incremental Updates**: Compare `mtime` to skip unchanged files.
- **Parameterized Queries**: Query compiler uses parameterized queries to prevent SQL injection.
- **Thread Safety**: Uses `Mutex<Database>` for thread-safe access in multi-threaded contexts.

## 8. Development Commands

See [README.md](./README.md#development) for usage examples and [README.md#testing] for test execution.

## 9. Project Structure

See [README.md](./README.md#project-structure) for the complete project structure.

## 10. Development Status

### Completed ✅
- Core indexing functionality
- Query system (SQL-like expressions)
- Field-based queries (reserved fields + frontmatter, simplified resolution)
- Query operators (==, !=, >, <, >=, <=, =~, =)
- Logical operators (and, or) with precedence
- has() function for array containment
- Multiple output formats (table, json, list)
- Backlink tracking
- Rust migration complete
- CLI with clap derive macros
- Incremental updates via mtime comparison
- File watching mode (--watch) for auto-reindexing
- Note creation with templates (new command)
- Single equals operator (=) support
- Database lock release in watch mode for concurrent queries
- Simplified field resolution (no file.*/note.* namespaces)
- Frontmatter conflict warnings (reserved fields except tags)

### Technical Debt / Future Improvements
- ✅ ~~Add unit tests for tokenizer, parser, and compiler~~ (Completed - 95 tests total)
- Add integration tests for full query pipeline
- Benchmark performance against 10,000 files goal
- Consider parallel processing for indexing
- Add configuration file support
- Implement query result caching

### Test Coverage Summary

**Unit Tests Implemented (95 tests total):**

| Module | Tests | Coverage |
|--------|-------|----------|
| `tokenizer.rs` | 13 | Field tokenization, operators, literals, functions, parentheses |
| `parser.rs` | 13 | Expression parsing, operators, grouping, precedence, single equals |
| `compiler.rs` | 17 | SQL generation, field resolution, all operators |
| `extractor.rs` | 18 | Frontmatter, tags, wiki-links, embeds, edge cases |
| `db.rs` | 12 | Database operations, queries, CRUD |
| `scanner.rs` | 10 | File scanning, indexing, backlinks, subdirectories |
| `query/mod.rs` | 10 | Output formatting (table, JSON, list) |
| `main.rs` | 2 | CLI options, default values, parsing |

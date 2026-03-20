# Markbase 综合修改计划

> 基于架构 Review、测试 Review、文档一致性 Review 的综合改进计划。
> 按优先级分为三个 Phase，每个任务包含具体文件、改动位置和验收标准。

---

## 总览

| Phase | 主题 | 任务数 | 预估工作量 |
|-------|------|--------|-----------|
| Phase 1 — 紧急修复 | Bug 修复 + 安全规范 | 3 | 0.5 天 |
| Phase 2 — 代码质量 | 重构 + 测试补全 | 5 | 2~3 天 |
| Phase 3 — 文档同步 | 文档与实现对齐 | 4 | 0.5 天 |

---

## Phase 1 — 紧急修复（Bug / 规范违反）

> 这些问题影响生产行为，应立即处理，每项可单独提 PR。

---

### Task 1.1：删除 `filter.rs` 中的 DEBUG 日志泄漏

**问题：** `src/renderer/filter.rs` 存在两个版本的 `translate_string_filter` 函数（函数重复定义），其中一个版本含有残留的调试输出：

```rust
eprintln!("DEBUG: translated_value = '{}'", translated_value);
```

这行代码在生产运行时会将调试信息输出到 stderr，污染所有依赖 stderr 清洁的测试和使用场景（如脚本中捕获 stderr 判断 WARN）。

**影响文件：** `src/renderer/filter.rs`

**具体改动：**

1. 定位文件中两处 `translate_string_filter` 函数定义（文件中前后各一份，后者是带 DEBUG 日志的版本）。
2. 保留**不含** `eprintln!("DEBUG:")` 的那一份（即前者，也是逻辑更完整的版本），**完整删除**含 DEBUG 的重复版本。
3. 确认保留的版本包含所有 filter 分支：`HAS_LINK_RE`、`HAS_TAG_RE`、`IN_FOLDER_RE`、`FILE_PROP_RE`、`IS_EMPTY_RE`、`CONTAINS_RE`、`COMPARE_RE`，且末尾有 `warnings.push(...)` 兜底。
4. 同时删除文件顶部因重复代码导致冗余的 `#![allow(...)]` suppress 项（如 `clippy::redundant_pattern_matching`），保留仍然必要的项。

**验收：**
```bash
cargo clippy -- -D warnings   # 无新 warning
grep -n "DEBUG" src/renderer/filter.rs  # 0 行输出
cargo test                    # 全部通过
```

---

### Task 1.2：修复 `main.rs` 中残留的 `.unwrap()` 调用

**问题：** `src/main.rs` 的 `TemplateCommands::List` 和 `Commands::Query` 分支违反了 AGENTS.md §13.1 的规范（非测试代码禁止使用 `.unwrap()`），而同文件的 `Verify`/`Render` 分支已正确使用 `.map_err()`：

```rust
// TemplateCommands::List — 违规
let db_ref = db.lock().unwrap();

// Commands::Query — 违规
let db = db.lock().unwrap();
```

**影响文件：** `src/main.rs`

**具体改动：**

将上述两处 `.unwrap()` 替换为与其他命令分支一致的写法：

```rust
// 修改前
let db_ref = db.lock().unwrap();

// 修改后
let db_ref = db.lock().map_err(|e| format!("failed to acquire db lock: {}", e))?;
```

两处均按此模式修改，变量名保持原样（`db_ref` / `db`）。

**验收：**
```bash
grep -n "\.unwrap()" src/main.rs  # 仅剩测试代码中的 unwrap（#[cfg(test)] 块内）
cargo clippy -- -D warnings
cargo test
```

---

### Task 1.3：为所有 happy-path 集成测试补充 stderr 空断言

**问题：** 集成测试只验证"正常情况下 stderr 应包含某内容"，从未验证"正常情况下 stderr 应为空"，导致 Task 1.1 中的 DEBUG 日志泄漏在测试阶段无法被发现。

**影响文件：** `tests/cli_note.rs`、`tests/cli_index.rs`（以及其他集成测试文件）

**具体改动：**

在所有"成功路径"测试（即调用 `assert_cli_success` 且不期望有 WARN 的测试）末尾，补充 stderr 为空的断言：

```rust
// 以 test_note_render_no_base_embeds 为例，增加：
let stderr = String::from_utf8_lossy(&output.stderr);
assert!(
    stderr.is_empty(),
    "unexpected stderr output: {}",
    stderr
);
```

需要补充该断言的测试（非完整列表，以实际无预期 WARN 的 happy-path 为准）：

- `test_note_render_no_base_embeds`
- `test_note_render_link_this_filter`
- `test_note_render_table_format`
- `test_note_render_empty_results`
- `test_note_render_dry_run`
- `test_note_verify_note_not_found`（错误路径，不适用）
- `test_index_creates_database`
- `test_index_with_wikilinks`

**验收：** 运行 `cargo test`，若 Task 1.1 未完成则此步骤的相关测试会失败——这正是期望的回归检测效果。

---

## Phase 2 — 代码质量改进

> 这些改动提高可维护性和测试覆盖率，建议按顺序完成，每项单独分支。

---

### Task 2.1：消除 `main.rs` 中冗余的 `Mutex` 包装

**问题：** 每个命令分支均独立执行 `Mutex::new(Database::open_existing(...))` 后立即 `.lock()`，单线程运行的 CLI 工具中 `Mutex` 无实际保护价值，且产生了模板代码噪音。

**影响文件：** `src/main.rs`

**具体改动方案（保守方案，最小侵入）：**

提取一个辅助函数 `open_db`，将"打开并锁定"封装为一步：

```rust
fn open_db(db_path: &std::path::Path) -> Result<Database, Box<dyn std::error::Error>> {
    Database::open_existing(db_path).map_err(|e| e.into())
}
```

然后将各命令分支中的：
```rust
let db = Mutex::new(Database::open_existing(&db_path)?);
let db = db.lock().map_err(|e| format!("failed to acquire db lock: {}", e))?;
```
替换为：
```rust
let db = open_db(&db_path)?;
```

涉及的命令分支：`Query`、`Note::Verify`、`Note::Render`、`Template::List`（Task 1.2 完成后）共 4 处。

**注意：** 若 `Database` 未实现 `Send`，则应先确认其是否可安全在单线程中直接持有。若实现有 `Mutex` 的原因是某些特殊约束，应在代码注释中说明，不做此重构。

**验收：**
```bash
grep -n "Mutex::new(Database" src/main.rs  # 0 行输出
cargo test
```

---

### Task 2.2：拆分 `renderer/mod.rs` 中过长的 `render_base_embed` 函数

**问题：** `render_base_embed` 函数在单一函数体内处理 YAML 解析、view 遍历、filter 翻译调用、SQL 拼接、查询执行、输出格式化，违反单一职责原则，难以测试和维护。

**影响文件：** `src/renderer/mod.rs`

**具体改动：**

将现有函数拆分为以下几个私有函数，`render_base_embed` 作为驱动函数调用它们：

```rust
/// 解析 .base 文件内容，返回全局 filter、properties、views 列表
fn parse_base_file(content: &str) -> Option<(Option<Value>, Option<Value>, Vec<Value>)>

/// 根据单个 view 和 ThisContext 构建完整 SQL 字符串
/// 返回 (sql, columns, warnings)
fn build_view_sql(
    view: &Value,
    global_filter: Option<&Value>,
    base_properties: Option<&Value>,
    this: &ThisContext,
    embed_name: &str,
) -> (String, Vec<ColumnMeta>, Vec<String>)

/// 执行查询并输出结果（list 或 table 格式）
fn execute_and_render(
    db: &Database,
    sql: &str,
    columns: &[ColumnMeta],
    view_name: &str,
    embed_name: &str,
    opts: &RenderOptions,
)
```

重构后 `render_base_embed` 的主体应缩减至约 20 行的驱动逻辑。

**验收：**
- `render_base_embed` 函数体行数 ≤ 30 行
- 现有集成测试（`test_note_render_*`）全部通过，无行为变化
- `cargo clippy -- -D warnings` 无新 warning

---

### Task 2.3：补全 `renderer/output.rs` 的单元测试

**问题：** `output.rs` 是纯函数模块（无 DB、无 IO），是单元测试性价比最高的地方，但测试存在状态不明的情况。计划文档中列出了 7 个应写的测试，需确认实际存在并补全。

**影响文件：** `src/renderer/output.rs`

**需要确认或新增的测试（在文件末尾的 `#[cfg(test)]` 块中）：**

```rust
#[test]
fn test_render_list_basic() {
    // 验证普通字段输出格式：每行 "display_name: value"
    let cols = vec![ColumnMeta { sql_expr: "name".into(), display_name: "name".into(), is_name_col: false, is_list_col: false }];
    let rows = vec![vec![("name".into(), Some("foo".into()))]];
    let out = render_list(&rows, &cols);
    assert!(out.contains("name: foo"));
    assert!(out.starts_with("---\n"));
}

#[test]
fn test_render_list_name_col() {
    // is_name_col=true 时值以 [[value]] 形式输出
}

#[test]
fn test_render_list_list_col() {
    // is_list_col=true 时值按 "- item" 多行展开
}

#[test]
fn test_render_list_empty() {
    // 空行集输出 "(no results)\n"
    let out = render_list(&[], &[]);
    assert_eq!(out, "(no results)\n");
}

#[test]
fn test_render_table_basic() {
    // 验证表头行、分隔行、数据行的 Markdown 表格格式
}

#[test]
fn test_render_table_list_col() {
    // is_list_col 字段在表格中用逗号拼接
}

#[test]
fn test_render_table_empty() {
    // 空结果集输出表头 + 分隔行 + "(no results)" 行
}
```

**验收：** `cargo test renderer::output` 全部通过。

---

### Task 2.4：补全 `renderer/filter.rs` 的边界条件单元测试

**问题：** 现有 filter 单元测试覆盖了主要翻译规则，但缺少以下关键边界场景，这些场景在实际 base 文件中很容易出现。

**影响文件：** `src/renderer/filter.rs`（在现有 `#[cfg(test)]` 块中追加）

**需要新增的测试：**

```rust
#[test]
fn test_translate_bare_column_not_direct_db_col() {
    // bare "name" 在 translate_columns 中必须翻译为 json_extract_string，
    // 不能直接映射为 name 数据库列
    let cols = translate_columns(&[serde_json::json!("name")], None, "t.base", &mut vec![]);
    assert!(cols[0].sql_expr.contains("json_extract_string"));
    assert!(!cols[0].is_name_col); // bare name 不是 file.name，不应标记为 name_col
}

#[test]
fn test_translate_sort_invalid_direction_warns_and_defaults_asc() {
    // direction 非 ASC/DESC 时，应输出 WARN 并默认使用 ASC
    let sort = serde_json::json!([{"property": "file.name", "direction": "INVALID"}]);
    let mut warnings = vec![];
    let result = translate_sort(Some(&sort), "t.base", &mut warnings);
    assert!(result.contains("ASC"));
    assert!(!warnings.is_empty());
}

#[test]
fn test_sql_injection_single_quote_escaping() {
    // note name 含单引号时必须正确转义为 ''
    let mut ctx = ctx();
    ctx.name = "O'Brien".to_string();
    let filter = serde_json::json!("related_customer == link(this)");
    let mut warnings = vec![];
    let result = translate_filter(&filter, &ctx, "t.base", &mut warnings);
    let sql = result.unwrap();
    assert!(sql.contains("O''Brien"), "Single quote must be escaped: {}", sql);
    assert!(!sql.contains("O'Brien\""), "Unescaped quote found: {}", sql);
}

#[test]
fn test_empty_and_array_returns_none() {
    // and: [] 为空数组时，整个 filter 应返回 None（无条件），不应 panic
    let filter = serde_json::json!({"and": []});
    let mut warnings = vec![];
    let result = translate_filter(&filter, &ctx(), "t.base", &mut warnings);
    assert!(result.is_none());
}

#[test]
fn test_views_empty_silently_skipped() {
    // translate_columns 对空 order 数组返回默认三列
    let cols = translate_columns(&[], None, "t.base", &mut vec![]);
    assert_eq!(cols.len(), 3);
}
```

**验收：** `cargo test renderer::filter` 全部通过，且 `cargo test` 整体通过。

---

### Task 2.5：改进 `query/mod.rs` 中形式主义测试

**问题：** `query/mod.rs` 的测试只断言 `result.is_ok()`，无法捕获输出内容错误（如输出空字符串也能通过）。

**影响文件：** `src/query/mod.rs`

**具体改动：**

针对核心格式化函数，将现有测试改为验证实际输出内容：

```rust
#[test]
fn test_output_table_header_and_separator() {
    // 验证表头行和分隔行实际存在于输出中
    // 需要捕获 stdout，可通过将输出函数改为返回 String 或用 capture 方式
    let results = vec![vec!["path1".to_string()]];
    let fields = vec!["path".to_string()];
    // 建议重构 output_table 接受 writer 参数（见注释），
    // 或在测试中验证文件/buffer 内容
}

#[test]
fn test_output_json_has_results_key() {
    // 验证 JSON 输出包含 "results" 键和正确的 count
}

#[test]
fn test_output_list_has_separator() {
    // 验证 list 输出中每条记录后有 "---" 分隔符
}
```

**备注：** 若 `output_table` 等函数直接写 `println!` 而无法注入 writer，则需先做小幅重构（传入 `impl Write` 参数），再补充测试。这是一个建议改进项，可与架构重构一同完成。

---

## Phase 3 — 文档同步

> 所有改动仅涉及 Markdown 文件，可在一个 PR 中完成。

---

### Task 3.1：更新 `AGENTS.md §10` 项目目录树

**问题：** §10 的目录树是旧版快照，缺少 `verifier.rs`、`renderer/`、`describe.rs`、`constants.rs`，会误导 coding agent。

**影响文件：** `AGENTS.md`

**具体改动：**

将 §10 Project Structure 中的 `src/` 目录树替换为：

```
src/
├── main.rs          # CLI entry point, argument parsing and command dispatch
├── lib.rs           # Library exports
├── constants.rs     # Shared constants (DB schema, field names, etc.)
├── db.rs            # DuckDB connection management, schema initialization, CRUD
├── scanner.rs       # index command driver, directory traversal, incremental update
├── extractor.rs     # Single file parsing: frontmatter, wiki-links, tags
├── creator.rs       # note new command, template rendering
├── renamer.rs       # note rename command, link updates
├── verifier.rs      # note verify command, MTS schema validation
├── describe.rs      # template describe command
├── renderer/
│   ├── mod.rs       # note render command, .base embed expansion pipeline
│   ├── filter.rs    # Base filter → DuckDB SQL translation; column/sort translation
│   └── output.rs    # list / table output formatting; ColumnMeta definition
└── query/
    ├── mod.rs       # Output formatting (table/json/list)
    ├── detector.rs  # SQL/expression mode detection, security validation
    ├── translator.rs # Field name translation
    ├── error_map.rs # DuckDB error mapping
    └── executor.rs  # Query execution orchestration
```

同时将 §10 中 spec 文档表格的 `template_schema.md` 描述从 `MTS v1.10` 改为 `MTS v1.11`。

---

### Task 3.2：更新 `AGENTS.md §11` Development Status 和 Test Coverage

**问题：** `note render` 命令未出现在 Completed 列表；`renderer/` 和 `verifier.rs` 未出现在 Test Coverage 表格。

**影响文件：** `AGENTS.md`

**具体改动：**

在 `Completed ✅` 列表末尾追加：
```markdown
- Note rendering with .base embed expansion (note render)
```

在 `Test Coverage` 表格末尾追加：
```markdown
| `verifier.rs`         | Note not found, no templates, location mismatch, required fields, type/enum/link validation |
| `renderer/filter.rs`  | Filter translation, column/sort translation, ThisContext, merge_filters |
| `renderer/output.rs`  | list/table format, is_name_col, is_list_col, empty results |
| `renderer/mod.rs`     | CLI integration: note render happy path, dry-run, base not found, link(this) |
```

同时将 `Technical Debt` 中的 `Integration test coverage` 更新为更精确的描述：
```markdown
- Unit test output content verification (query/mod.rs)
- Negative stderr assertions in integration tests
```

---

### Task 3.3：更新 `docs/references/legacy-designs/note_render_design.md` 使其与实现一致

**问题：** spec 中的输出格式示例与实际实现存在三处偏差。

**影响文件：** `docs/references/legacy-designs/note_render_design.md`

**具体改动：**

**① 更新输出注释格式**

将示例中的：
```
<!-- [markbase] rendered from customer-opportunities.base -->
```
替换为实际实现的格式：
```
<!-- start: [markbase] rendered from customer-opportunities.base -->
...
<!-- end: [markbase] rendered from customer-opportunities.base -->
```

对 `--dry-run` 格式做同样更新（`dry-run from` → `start: [markbase] dry-run from ... / end: ...`）。

**② 更新 list 格式输出示例**

将示例中的裸文本 list 输出：
```
---
name: [[绿米-商机2026]]
stage: Proposal
```
更新为实际代码中带 yaml 代码块包裹的格式：
````
```yaml
---
name: [[绿米-商机2026]]
stage: Proposal
mtime: 2026-02-10
```
````

**③ 补充 `.base` 文件直接渲染的说明**

在"命令格式"或"执行流程"章节补充一个说明段落：

> **直接渲染 `.base` 文件：** 若 `<n>` 参数对应的是一个 `.base` 文件（即 `name` 字段含扩展名如 `opps.base`），命令将跳过正文扫描步骤，直接执行该 `.base` 文件中的所有 view，等效于该文件作为独立 base 被渲染。

---

### Task 3.4：更新 `AGENTS.md §13` 中不完整的 Rust 规范

**问题：** AGENTS.md §13 标题写的是 "§14. Rust Best Practices" 但节号是 "## 14"，同时 §13.1 等子节号与外层 §14 不一致（历史编号混乱）。同时，规范中提到"使用 `thiserror`"，但实际代码大量使用 `Box<dyn std::error::Error>`，应补充说明何时用哪种。

**影响文件：** `AGENTS.md`

**具体改动：**

在 §13.1 Error Handling 中补充一条说明：

```markdown
- Use `Box<dyn std::error::Error>` for command-level errors propagated to `main()` (consistent with existing modules)
- Reserve `thiserror` for structured error types that need to be matched by callers (currently none in this codebase)
```

修正节号：将 `## 14. Rust Best Practices` 改为 `## 13. Rust Best Practices`（或按实际编号统一）。

---

## 执行顺序建议

```
Phase 1（必须先做，阻塞后续测试）
  └── Task 1.1  删除 DEBUG 日志（filter.rs）         ← 最优先，有 stderr 污染
  └── Task 1.2  修复 .unwrap()（main.rs）            ← 规范合规，5 分钟改动
  └── Task 1.3  补充 stderr 空断言（集成测试）        ← Task 1.1 完成后立即加

Phase 2（建议顺序执行）
  └── Task 2.3  补全 output.rs 单元测试              ← 无依赖，可并行
  └── Task 2.4  补全 filter.rs 边界测试              ← 无依赖，可并行
  └── Task 2.1  消除 Mutex 冗余包装                  ← 依赖 Task 1.2
  └── Task 2.2  拆分 render_base_embed              ← 依赖 Task 2.3/2.4 先建立测试保护
  └── Task 2.5  改进 query/mod.rs 测试              ← 独立，可任意时机

Phase 3（文档，最后做，不阻塞代码）
  └── Task 3.1 + 3.2 + 3.3 + 3.4  合并为一个 PR
```

---

## 验收总清单

完成所有任务后，执行以下完整检查：

```bash
# 1. 代码规范
cargo fmt --check
cargo clippy -- -D warnings

# 2. 无 DEBUG 输出泄漏
grep -rn "eprintln!.*DEBUG" src/

# 3. 非测试代码无 .unwrap()
grep -rn "\.unwrap()" src/ | grep -v "#\[cfg(test)\]" | grep -v "// safe:"

# 4. 测试全部通过
cargo test

# 5. 构建成功
cargo build --release
```

所有命令零输出 / 零 warning / 零失败为验收通过。

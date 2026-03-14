# `note render` 命令实现计划

**面向：** Coding Agent  
**参考设计：** `docs/design-docs/legacy/note_render_design.md`（必须先阅读）  
**影响文件：**
- `src/renderer/mod.rs`（新增）— 核心渲染逻辑
- `src/renderer/filter.rs`（新增）— Filter 翻译、ThisContext、列名翻译、sort 翻译
- `src/renderer/output.rs`（新增）— list / table 格式化输出
- `src/lib.rs` — 追加 `pub mod renderer`
- `src/main.rs` — 追加 `mod renderer`，注册子命令
- `tests/cli_note.rs` — 端到端测试
- `tests/common.rs` — 辅助方法
- `README.md`、`AGENTS.md`、`CHANGELOG.md` — 文档更新

**在开始前必须阅读：**
1. `AGENTS.md` — 项目规范（禁止 `.unwrap()`、输出路由、退出码规则等）
2. `docs/design-docs/legacy/note_render_design.md` — 所有翻译规则的权威来源
3. `src/db.rs` — 了解 `db.query()` 接口签名和 `QueryResult` 类型
4. `src/main.rs` — 了解现有 `NoteCommands` 枚举结构和 match 分支写法，参照 `Verify` 分支

---

## Rust 模块结构说明（重要）

`src/renderer/` 是子模块**目录**，入口文件为 `src/renderer/mod.rs`。
**不能**同时存在 `src/renderer.rs` 和 `src/renderer/` 目录——只创建目录结构。

需要创建的三个文件：
```
src/renderer/mod.rs      ← Task 3（主逻辑）
src/renderer/filter.rs   ← Task 2（翻译逻辑）
src/renderer/output.rs   ← Task 1（格式化）
```

---

## 总体顺序

```
Task 0  创建 feature branch
Task 1  新增 src/renderer/output.rs    （格式化输出，ColumnMeta 定义，无外部依赖）
Task 2  新增 src/renderer/filter.rs    （filter/列名/sort 翻译，依赖 output 的 ColumnMeta）
Task 3  新增 src/renderer/mod.rs       （渲染主逻辑，依赖 filter + output）
Task 4  修改 src/lib.rs 和 src/main.rs （模块导出 + CLI 注册，两个文件都要改）
Task 5  补充测试
Task 6  更新文档
Task 7  pre-commit 检查并提交
```

---

## Task 0：创建 Feature Branch

```bash
git checkout -b feat/note-render
```

---

## Task 1：新增 `src/renderer/output.rs`

该模块只负责格式化，不涉及数据库或 YAML 解析，先写便于独立单元测试。

### 1.1 类型定义

```rust
/// 一列的元数据，由 filter.rs 的 translate_columns 生成，由 output.rs 使用
#[derive(Debug, Clone)]
pub struct ColumnMeta {
    pub sql_expr: String,      // SELECT 子句中使用的 SQL 表达式
    pub display_name: String,  // 输出时显示的列名（来自 properties.displayName 或默认值）
    pub is_name_col: bool,     // true 时值以 [[value]] 输出（仅 file.name 列为 true）
    pub is_list_col: bool,     // true 时值是数组（file.tags/file.links/file.embeds 为 true）
}

/// 一行数据：每项是 (display_name, 值)
/// 值为 None 表示该列在本行为空
pub type Row = Vec<(String, Option<String>)>;
```

### 1.2 render_list 函数

```rust
/// 渲染 list 格式（key-value 块）
pub fn render_list(rows: &[Row], columns: &[ColumnMeta]) -> String
```

输出规则：
- 空结果集（`rows` 为空）→ 输出 `"(no results)\n"`
- 每条记录以 `"---\n"` 开头
- 每字段一行：`display_name: value`
- `is_name_col == true` 且值非空 → 值以 `[[value]]` 形式输出
- `is_list_col == true` → 值字符串形如 `[a, b, c]`（DuckDB 数组列的字符串表示），解析后每元素独占一行：
  ```
  display_name:
    - a
    - b
  ```
  解析方式：去掉首尾 `[` `]`，按 `, ` 分割。

### 1.3 render_table 函数

```rust
/// 渲染 table 格式（Markdown 表格）
pub fn render_table(rows: &[Row], columns: &[ColumnMeta]) -> String
```

输出规则：
- 表头行：`| display_name | display_name | ...`
- 分隔行：`|---|---|...`
- 数据行：`is_list_col` 字段所有元素用 `, ` 拼接；`is_name_col` 字段值加 `[[]]`
- 空结果集 → 输出表头 + 分隔行 + `| (no results) |` 行

### 1.4 单元测试（同文件 `#[cfg(test)]`）

```rust
fn test_render_list_basic()           // 普通字段输出格式
fn test_render_list_name_col()        // is_name_col → [[value]]
fn test_render_list_list_col()        // is_list_col → 多行 - 格式
fn test_render_list_empty()           // 空行集 → "(no results)"
fn test_render_table_basic()          // 表格基本结构
fn test_render_table_list_col()       // 数组字段逗号拼接
fn test_render_table_empty()          // 空结果含表头
```

---

## Task 2：新增 `src/renderer/filter.rs`

该模块包含所有翻译逻辑。**所有规则以 `docs/design-docs/legacy/note_render_design.md` §Filter 翻译规范为权威，遇到歧义以设计文档为准。**

### 2.1 依赖声明

```rust
use crate::renderer::output::ColumnMeta;
use regex::Regex;
use std::sync::LazyLock;
use serde_json::Value;
```

### 2.2 ThisContext 定义

```rust
/// render 执行时，note <n> 的数据库行快照，用于解析 this.* / link(this)
#[derive(Debug, Clone)]
pub struct ThisContext {
    pub path:   String,   // 列索引 0
    pub folder: String,   // 列索引 1，如 "company/"，根目录笔记为空字符串
    pub name:   String,   // 列索引 2，不含扩展名，就是命令行传入的 <n>
    pub ext:    String,   // 列索引 3
    pub size:   i64,      // 列索引 4，从字符串 parse，失败时用 0
    pub ctime:  String,   // 列索引 5，文本形式
    pub mtime:  String,   // 列索引 6，文本形式
    pub tags:   Vec<String>, // 列索引 7，由 to_json() 返回的 JSON 数组字符串解析
    pub links:  Vec<String>, // 列索引 8，同上
}
```

`ThisContext` 的构建方式（在 `mod.rs` Step 0 中）：
```rust
let row = &rows[0]; // rows 来自 db.query() 的第二个元素 Vec<Vec<String>>
let this = ThisContext {
    path:   row[0].clone(),
    folder: row[1].clone(),
    name:   row[2].clone(),
    ext:    row[3].clone(),
    size:   row[4].parse().unwrap_or(0),
    ctime:  row[5].clone(),
    mtime:  row[6].clone(),
    tags:   serde_json::from_str(&row[7]).unwrap_or_default(),
    links:  serde_json::from_str(&row[8]).unwrap_or_default(),
};
```

### 2.3 translate_filter（公开）

```rust
/// 将一个 filter Value 翻译为 DuckDB WHERE 子句片段（不含 WHERE 关键字）
/// 返回 None 表示整个 filter 无法翻译（所有子条件均不支持），调用方应省略该条件
pub fn translate_filter(
    filter: &Value,
    this: &ThisContext,
    base_name: &str,
    warnings: &mut Vec<String>,
) -> Option<String>
```

**实现逻辑：**

```
match filter:
  Value::Object 且含 "and" 键 →
    取 "and" 值作为 Value::Array，遍历每个元素递归调用 translate_filter
    过滤掉 None，剩余用 " AND " 拼接，包裹为 "(...)"
    若全部为 None 或数组为空 → 返回 None

  Value::Object 且含 "or" 键 → 同上，用 " OR " 拼接

  Value::Object 且含 "not" 键 →
    取数组第一个元素递归翻译，结果包裹为 "NOT (...)"
    若元素为 None → 返回 None

  Value::String(s) → 调用 translate_string_filter(s, ...)

  其他 → 追加 WARN，返回 None
```

注意：同一个 `and`/`or` 数组里，每个元素既可能是 `Value::String`（字符串条件），也可能是 `Value::Object`（嵌套的 and/or/not）。递归调用时统一判断类型。

### 2.4 translate_string_filter（私有）

```rust
fn translate_string_filter(
    s: &str,
    this: &ThisContext,
    base_name: &str,
    warnings: &mut Vec<String>,
) -> Option<String>
```

**实现步骤（按顺序，命中即返回）：**

**Step A：预处理 link() 替换**

在对字符串做任何模式匹配之前，先对整个字符串做两次文本替换：
- `link(this)` → `"[[<this.name>]]"`（双引号包裹的 wikilink 字面量）
- `link("x")` → `"[[x]]"`（对任意 x，用正则 `link\("([^"]+)"\)` 匹配提取）

替换后的字符串用于后续所有匹配。

**Step B：文件函数匹配**

用正则逐一尝试：

1. `file.hasLink(this.file)` — 精确字符串匹配（预处理后 `this.file` 保持不变）
   - 翻译：`list_contains(links, '<this.name>')`

2. `file.hasTag(...)` — 正则 `^file\.hasTag\((.+)\)$`，提取括号内容按逗号分割参数
   - 每个参数去掉引号，单参数翻译：`(list_contains(tags, 't1') OR array_any(tags, x -> x LIKE 't1/%'))`
   - 多参数：各自翻译后 OR 拼接，整体加括号

3. `file.inFolder("x")` — 正则 `^file\.inFolder\("([^"]+)"\)$`
   - 规范化文件夹名：去掉末尾 `/` 再加 `/`（统一加尾斜杠）
   - 翻译：`(folder = 'x/' OR folder LIKE 'x/%')`

4. `file.<prop> <op> <expr>` — 正则 `^file\.(\w+)\s*([><=!]+)\s*(.+)$`
   - `<prop>` 映射到列名（见设计文档列名翻译速查表）
   - `<expr>` 先尝试日期表达式翻译（见 Step E），失败则作字符串字面量处理
   - 翻译：`<col> <op> <translated_expr>`

**Step C：方法调用匹配**

5. `<attr>.isEmpty()` — 正则 `^(.+)\.isEmpty\(\)$`
   - 解析 `<attr>` 前缀（`file.tags`、`note.<f>`、bare `<f>`）
   - 翻译见设计文档 §3.4

6. `<attr>.contains(<arg>)` — 正则 `^(.+)\.contains\((.+)\)$`
   - `<arg>` 已经过 Step A 预处理，若形如 `"[[...]]"` 则提取 wikilink 内容
   - 统一翻译为 `list_contains((properties->'$."<f>"')::VARCHAR[], '<val>')`

**Step D：比较运算符匹配**

7. `<attr> <op> <value>` — 正则 `^(.+?)\s*([><=!]+)\s*(.+)$`
   - 解析 `<attr>` 前缀，翻译为对应 SQL 表达式
   - 解析 `<value>`：
     - 形如 `"[[...]]"` 或 `'[[...]]'` → 去外层引号，翻译为 `= '[[...]]'`（单引号内含单引号时转义）
     - 形如带引号的字符串（`"v"` 或 `'v'`）→ 去引号，内部单引号 `'` → `''` 转义
     - 纯数字 → 加 `::DOUBLE` cast
   - 翻译：`<sql_expr> <op> <translated_value>`

**Step E：日期表达式翻译子函数**

```rust
fn translate_date_expr(expr: &str) -> Option<String>
```

识别并翻译以下模式：
- `now()` → `NOW()`
- `today()` → `CURRENT_DATE`
- `now() - "30d"` 或 `now() - "1 year"` → `NOW() - INTERVAL '<n> <UNIT>'`
- `today() + "1M"` → `CURRENT_DATE + INTERVAL '1 MONTH'`

持续时间字符串解析正则：`^(\d+)\s*([a-zA-Z]+)$`，单位词映射：

| 单位词 | INTERVAL 单位 |
|---|---|
| `y` / `year` / `years` | `YEAR` |
| `M` / `month` / `months` | `MONTH` |
| `w` / `week` / `weeks` | `WEEK` |
| `d` / `day` / `days` | `DAY` |
| `h` / `hour` / `hours` | `HOUR` |
| `m` / `minute` / `minutes` | `MINUTE` |
| `s` / `second` / `seconds` | `SECOND` |

**Step F：无法匹配**

追加 WARN，返回 None：
```
WARN: unsupported filter '<s>' in '<base_name>.base', condition ignored.
```

### 2.5 merge_filters（公开）

```rust
/// 将全局 filter 和 view 级 filter 合并为 WHERE 子句（不含 WHERE 关键字）
pub fn merge_filters(
    global: Option<&Value>,
    view: Option<&Value>,
    this: &ThisContext,
    base_name: &str,
    warnings: &mut Vec<String>,
) -> Option<String>
```

合并规则：
- 两者都翻译后用 AND 拼接：`(<global>) AND (<view>)`
- 任一为 None 时只取另一个
- 两者均为 None → 返回 None

### 2.6 translate_columns（公开）

```rust
/// 将 view.order（字符串列表）翻译为 Vec<ColumnMeta>
/// order_vals：来自 base YAML 中 view["order"]，类型为 &[Value]（可为空切片）
/// properties：来自 base YAML 中顶层的 "properties" 字段
pub fn translate_columns(
    order_vals: &[Value],
    properties: Option<&Value>,
    base_name: &str,
    warnings: &mut Vec<String>,
) -> Vec<ColumnMeta>
```

**列名解析规则（完整映射表）：**

| order 中的字符串 | sql_expr | display_name | is_name_col | is_list_col |
|---|---|---|---|---|
| `file.name` | `name` | `name` | **true** | false |
| `file.path` | `path` | `path` | false | false |
| `file.mtime` | `mtime` | `mtime` | false | false |
| `file.ctime` | `ctime` | `ctime` | false | false |
| `file.size` | `size` | `size` | false | false |
| `file.ext` | `ext` | `ext` | false | false |
| `file.folder` | `folder` | `folder` | false | false |
| `file.tags` | `tags` | `tags` | false | **true** |
| `file.links` | `links` | `links` | false | **true** |
| `file.embeds` | `embeds` | `embeds` | false | **true** |
| `note.<f>` | `json_extract_string(properties, '$."<f>"')` | `<f>` | false | false |
| bare `<f>`（无任何前缀） | 同 `note.<f>` | `<f>` | false | false |
| `formula.*` | **跳过，WARN** | — | — | — |

> **关键规则**：bare 列名（如 `name`、`tags`、`status`）无论是否与数据库列名相同，**一律**按 `note.<f>` 翻译为 `json_extract_string`，不作为直接列名处理。

**默认列**：`order_vals` 为空时，返回：
```rust
vec![
    ColumnMeta { sql_expr: "name".into(), display_name: "name".into(), is_name_col: true, is_list_col: false },
    ColumnMeta { sql_expr: "path".into(), display_name: "path".into(), is_name_col: false, is_list_col: false },
    ColumnMeta { sql_expr: "mtime".into(), display_name: "mtime".into(), is_name_col: false, is_list_col: false },
]
```

**displayName 查找**：`properties[<order中的列名字符串>]["displayName"]`，键名与 order 字符串**完全相同**（含前缀，如 `"file.ext"` 对应键 `"file.ext"`，`"status"` 对应键 `"status"`）。找到时用 displayName 值覆盖默认 display_name。

### 2.7 translate_sort（公开）

```rust
/// 将 view.sort（对象列表）翻译为 ORDER BY 子句（不含 ORDER BY 关键字）
/// sort_val：来自 base YAML 中 view["sort"]，类型为 Option<&Value>
/// 返回空字符串表示无排序（省略 ORDER BY）
pub fn translate_sort(
    sort_val: Option<&Value>,
    base_name: &str,
    warnings: &mut Vec<String>,
) -> String
```

每个 sort 元素结构：`{"property": "file.name", "direction": "ASC"}`

- `property` 的列名翻译规则与 `translate_columns` 完全相同（同一个内部函数）
- `direction`：大小写不敏感，只接受 `"ASC"` 或 `"DESC"`，其他值 WARN 后默认 `ASC`
- `sort_val` 为 None 或数组为空 → 返回 `""`

### 2.8 单元测试（同文件 `#[cfg(test)]`）

```rust
fn test_translate_link_this_equality()        // "related_customer == link(this)" 翻译
fn test_translate_list_contains_link_name()   // "templates.contains(link(\"activity_log\"))"
fn test_translate_list_contains_link_this()   // "field.contains(link(this))"
fn test_translate_has_link_this_file()        // "file.hasLink(this.file)"
fn test_translate_has_tag_single()            // 单 tag，含嵌套 LIKE 匹配
fn test_translate_has_tag_multi()             // 多参数 hasTag
fn test_translate_in_folder_no_slash()        // "notes" → folder = 'notes/'
fn test_translate_in_folder_with_slash()      // "notes/" → folder = 'notes/'
fn test_translate_date_30d()                  // "30d" → INTERVAL '30 DAY'
fn test_translate_date_1_year_spaced()        // "1 year" → INTERVAL '1 YEAR'
fn test_translate_date_ctime_expr()           // "file.ctime > now() - \"1 year\""
fn test_translate_is_empty_note()
fn test_translate_is_empty_file_tags()
fn test_translate_bare_equality()             // "status == \"done\"" → json_extract_string
fn test_translate_bare_not_direct_col()       // bare "name" → json_extract_string，不是 name 列
fn test_translate_nested_and_in_and()         // and 嵌套在 and 里（真实 base 结构）
fn test_translate_single_element_and()        // 单元素 and 列表
fn test_translate_mixed_string_and_object()   // and 数组中字符串与对象混合
fn test_merge_filters_both()
fn test_merge_filters_one_none()
fn test_merge_filters_both_none()
fn test_translate_columns_empty_uses_defaults()
fn test_translate_columns_bare_name_is_json_extract()  // bare "name" 不是直接列
fn test_translate_columns_bare_tags_is_json_extract()  // bare "tags" 不是 file.tags
fn test_translate_columns_file_tags_is_list()
fn test_translate_columns_file_links_is_list()
fn test_translate_columns_file_embeds_is_list()
fn test_translate_columns_display_name_override()
fn test_translate_sort_basic()
fn test_translate_sort_invalid_direction()    // 非 ASC/DESC → ASC + WARN
fn test_translate_sort_empty()
fn test_translate_sort_bare_property()        // bare property → json_extract_string
```

---

## Task 3：新增 `src/renderer/mod.rs`

渲染主模块，串联 Step 0–5。

### 3.1 文件头

```rust
pub mod filter;
pub mod output;

use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use gray_matter::Matter;
use gray_matter::engine::YAML;
use regex::Regex;
use serde_json::Value;

use crate::db::Database;
use crate::renderer::filter::{ThisContext, merge_filters, translate_columns, translate_sort};
use crate::renderer::output::{ColumnMeta, Row, render_list, render_table};
```

### 3.2 公开类型

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum RenderFormat { List, Table }

pub struct RenderOptions {
    pub format: RenderFormat,
    pub dry_run: bool,
}
```

### 3.3 render_note（公开主入口）

```rust
pub fn render_note(
    base_dir: &Path,
    db: &Database,
    name: &str,
    opts: &RenderOptions,
) -> Result<(), Box<dyn std::error::Error>>
```

禁止在非测试代码中使用 `.unwrap()` / `.expect()`，一律用 `?` 或 `map_err`。

### 3.4 Step 0：定位 note 并构建 ThisContext

```rust
// db.query 签名：
//   pub fn query(&self, sql: &str, _fields: &str, _limit: usize)
//              -> Result<QueryResult, Box<dyn std::error::Error>>
// QueryResult = (Vec<String>, Vec<Vec<String>>)，即 (列名列表, 行数据)
// 所有列值均以 String 返回，包括数值和时间戳列

let name_escaped = name.replace('\'', "''"); // 单引号转义，防止 SQL 错误
let sql = format!(
    "SELECT path, folder, name, ext, size, \
     CAST(ctime AS TEXT), CAST(mtime AS TEXT), \
     to_json(tags), to_json(links), properties \
     FROM notes WHERE name = '{}'",
    name_escaped
);
// usize::MAX 作为 limit，不套 executor.rs 的默认 1000
let (_, rows) = db.query(&sql, "", usize::MAX)
    .map_err(|e| format!("database query failed: {}", e))?;
```

- `rows` 为空 → stderr 输出 ERROR，返回 `Err`：
  ```
  ERROR: note '<n>' not found in index. Run `markbase index` first.
  ```
- `rows.len() > 1` → 同上报 ERROR（name 全局唯一，理论上不应发生）

构建 `ThisContext` 按列索引取值（见 Task 2 §2.2）。

note 文件路径：`base_dir.join(&this.path)`

### 3.5 Step 1：流式处理正文

```rust
let content = fs::read_to_string(base_dir.join(&this.path))?;
let matter = Matter::<YAML>::new();
let body = matter.parse(&content).content; // frontmatter 已剥离

static BASE_EMBED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^!\[\[([^\]]+\.base)\]\]\s*$").unwrap());

for line in body.lines() {
    if let Some(caps) = BASE_EMBED_RE.captures(line) {
        let embed_name = caps.get(1).unwrap().as_str(); // safe: group 1 必然存在
        render_base_embed(embed_name, base_dir, db, &this, opts);
    } else {
        println!("{}", line);
    }
}
```

### 3.6 render_base_embed（私有）

```rust
fn render_base_embed(
    embed_name: &str,   // 完整文件名，含扩展名，如 "opps.base"
    base_dir: &Path,
    db: &Database,
    this: &ThisContext,
    opts: &RenderOptions,
)
// 直接写 stdout/stderr，无返回值；所有错误内部处理，不向上传播
```

**Step 2：查询 base 文件路径**

```rust
// 注意：非 .md 文件的 name 列存储完整文件名含扩展名
// 查询时直接用 embed_name（如 "opps.base"），不去掉扩展名
let embed_escaped = embed_name.replace('\'', "''");
let sql = format!("SELECT path FROM notes WHERE name = '{}'", embed_escaped);
let result = db.query(&sql, "", usize::MAX);
```

- 查询失败或 `rows` 为空 →
  ```rust
  eprintln!("WARN: base file '{}' not found in index, skipping.", embed_name);
  println!("<!-- [markbase] base '{}' not found -->", embed_name);
  return;
  ```

读取并解析 YAML：
```rust
let base_path = base_dir.join(&rows[0][0]);
let base_content = match fs::read_to_string(&base_path) {
    Ok(c) => c,
    Err(e) => {
        eprintln!("WARN: failed to read '{}': {}", embed_name, e);
        println!("<!-- [markbase] failed to read '{}' -->", embed_name);
        return;
    }
};
let base_yaml: Value = match serde_yaml::from_str(&base_content) {
    Ok(v) => v,
    Err(e) => {
        eprintln!("WARN: failed to parse '{}': {}", embed_name, e);
        println!("<!-- [markbase] failed to parse '{}': {} -->", embed_name, e);
        return;
    }
};
```

提取字段：
```rust
let global_filter = base_yaml.get("filters");
let base_properties = base_yaml.get("properties");
let views = match base_yaml.get("views").and_then(|v| v.as_array()) {
    Some(v) if !v.is_empty() => v,
    _ => return, // views 不存在或为空，静默跳过，不输出任何内容
};
```

**Step 3**：`this` 已在 Step 0 构建完毕，直接传参使用。

**Step 4 + Step 5：逐 view 处理**

```rust
for view in views {
    let view_name = view.get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(embed_name);

    let order_vals: &[Value] = view.get("order")
        .and_then(|v| v.as_array())
        .map(|a| a.as_slice())
        .unwrap_or(&[]);

    let mut warnings: Vec<String> = Vec::new();

    let where_clause = merge_filters(
        global_filter, view.get("filters"), this, embed_name, &mut warnings
    );
    let columns = translate_columns(order_vals, base_properties, embed_name, &mut warnings);
    let order_by = translate_sort(view.get("sort"), embed_name, &mut warnings);

    // 所有 WARN 输出到 stderr
    for w in &warnings { eprintln!("{}", w); }

    // 构造 SQL
    let select_exprs: Vec<&str> = columns.iter().map(|c| c.sql_expr.as_str()).collect();
    let mut sql = format!("SELECT {} FROM notes", select_exprs.join(", "));
    if let Some(w) = &where_clause {
        sql.push_str(&format!(" WHERE {}", w));
    }
    if !order_by.is_empty() {
        sql.push_str(&format!(" ORDER BY {}", order_by));
    }
    if let Some(l) = view.get("limit").and_then(|v| v.as_u64()) {
        sql.push_str(&format!(" LIMIT {}", l));
    }

    // 输出（dry-run 或正常）
    if opts.dry_run {
        println!("<!-- [markbase] dry-run from {} -->\n", embed_name);
        println!("## {}\n", view_name);
        println!("```sql\n{}\n```", sql);
    } else {
        println!("<!-- [markbase] rendered from {} -->\n", embed_name);
        println!("## {}\n", view_name);

        match db.query(&sql, "", usize::MAX) {
            Ok((_, raw_rows)) => {
                // 将 Vec<Vec<String>> 转为 Vec<Row>
                // Row = Vec<(display_name, Option<String>)>
                // 空字符串视为 None（无值）
                let rows: Vec<Row> = raw_rows.iter().map(|raw| {
                    columns.iter().enumerate().map(|(i, col)| {
                        let val = raw.get(i).cloned()
                            .filter(|s| !s.is_empty());
                        (col.display_name.clone(), val)
                    }).collect()
                }).collect();

                let output = match opts.format {
                    RenderFormat::Table => render_table(&rows, &columns),
                    RenderFormat::List  => render_list(&rows, &columns),
                };
                print!("{}", output);
            }
            Err(e) => {
                eprintln!("WARN: query failed for view '{}' in '{}': {}", view_name, embed_name, e);
                println!("<!-- [markbase] query failed for view '{}' -->", view_name);
            }
        }
    }
}
```

---

## Task 4：修改 `src/lib.rs` 和 `src/main.rs`

**两个文件都必须修改。**

### 4.1 src/lib.rs

在现有 `pub mod` 列表末尾追加（参照已有的 `pub mod verifier;`）：

```rust
pub mod renderer;
```

### 4.2 src/main.rs — mod 声明

在文件顶部 mod 列表中追加（参照已有的 `mod verifier;`）：

```rust
mod renderer;
```

### 4.3 src/main.rs — NoteCommands 枚举

在 `Verify` 变体之后新增：

```rust
#[command(about = "Render a note to stdout, expanding .base embeds")]
Render {
    #[arg(help = "Note name (without .md extension)")]
    name: String,

    #[arg(short = 'o', help = "Output format: list (default) or table")]
    format: Option<OutputFormat>,

    #[arg(long = "dry-run", help = "Show SQL instead of executing queries")]
    dry_run: bool,
},
```

### 4.4 src/main.rs — match 分支

在 `NoteCommands::Verify { ... }` 分支之后新增，写法与 `Verify` / `Rename` 分支完全一致，**不使用 `.unwrap()`**：

```rust
NoteCommands::Render { name, format, dry_run } => {
    let base = get_base_dir_absolute_with_cli(cli.base_dir.clone())?;
    check_db_exists(&db_path, &base_dir)?;
    let db = Mutex::new(Database::open_existing(&db_path)?);
    let db = db.lock().map_err(|e| format!("failed to acquire db lock: {}", e))?;

    let render_format = match format.or(cli.output_format) {
        Some(OutputFormat::Table) => renderer::RenderFormat::Table,
        _ => renderer::RenderFormat::List, // List 为默认；Json 不支持，静默降级为 List
    };
    let opts = renderer::RenderOptions { format: render_format, dry_run };

    if let Err(e) = renderer::render_note(&base, &db, &name, &opts) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
```

---

## Task 5：补充测试

### 5.1 tests/common.rs 新增辅助函数

```rust
/// 在 dir 目录下创建一个文件，写入 content
/// 用于创建 .base 文件：create_file(&dir, "opps.base", "views:\n...")
pub fn create_file(dir: &Path, filename: &str, content: &str) {
    std::fs::write(dir.join(filename), content).unwrap();
}
```

### 5.2 tests/cli_note.rs 端到端测试

所有测试均按此流程：构建临时 vault → `markbase index` → `markbase note render` → 验证 stdout/stderr/exit code。

下面给出 3 个完整示例，其余参照相同模式：

**示例 1：核心用例 — `link(this)` filter**

```rust
#[test]
fn test_render_note_link_this_filter() {
    let dir = tempdir().unwrap();

    // 创建 company note（带 base 嵌入）
    create_file(dir.path(), "acme.md",
        "---\ntype: company\n---\n![[opps.base]]\n");

    // 创建 opportunity note，frontmatter 链接到 acme
    create_file(dir.path(), "deal1.md",
        "---\ntype: opportunity\nrelated_customer: \"[[acme]]\"\n---\n");

    // 创建 .base 文件
    create_file(dir.path(), "opps.base", "\
views:
  - type: table
    name: Opportunities
    filters:
      and:
        - related_customer == link(this)
    order:
      - file.name
");

    run_cli(&dir, &["index"]);
    let out = run_cli(&dir, &["note", "render", "acme"]);

    assert!(out.status.success());
    // 来源注释
    assert!(String::from_utf8_lossy(&out.stdout).contains("rendered from opps.base"));
    // file.name 列链接化
    assert!(String::from_utf8_lossy(&out.stdout).contains("[[deal1]]"));
}
```

**示例 2：dry-run 输出 SQL 并含 link(this) 翻译结果**

```rust
#[test]
fn test_render_note_dry_run() {
    // ... 同上构建 vault（acme.md + deal1.md + opps.base）
    run_cli(&dir, &["index"]);
    let out = run_cli(&dir, &["note", "render", "acme", "--dry-run"]);

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("dry-run from opps.base"));
    assert!(stdout.contains("FROM notes"));
    // link(this) 翻译为 [[acme]] 字面量
    assert!(stdout.contains("[[acme]]"));
}
```

**示例 3：note 不存在 → exit code 1**

```rust
#[test]
fn test_render_note_not_found() {
    let dir = tempdir().unwrap();
    run_cli(&dir, &["index"]);
    let out = run_cli(&dir, &["note", "render", "nonexistent"]);
    assert!(!out.status.success()); // exit code 1
    assert!(String::from_utf8_lossy(&out.stderr).contains("not found in index"));
}
```

**其余测试用例（按相同模式编写）：**

| 测试函数 | 场景 | 验证重点 |
|---|---|---|
| `test_render_note_no_base_embeds` | 正文无 `.base` 嵌入 | stdout 原样输出正文，exit 0 |
| `test_render_note_base_not_found` | base 文件未被 index | stderr 含 WARN，stdout 含占位注释，exit 0 |
| `test_render_note_table_format` | `-o table` | stdout 含 `\| name \|` 表格格式 |
| `test_render_note_date_filter` | `file.ctime > now() - "1 year"` | dry-run SQL 含 `INTERVAL '1 YEAR'` |
| `test_render_note_empty_results` | 查询无结果 | stdout 含 `(no results)` |
| `test_render_note_list_field` | order 含 `file.tags` | list 格式下 tags 每元素独占一行 |
| `test_render_note_sort` | view 含 `sort` 字段 | dry-run SQL 含 `ORDER BY` |
| `test_render_note_unsupported_filter` | filter 含 `this.note.*` | stderr 含 WARN，渲染继续，exit 0 |
| `test_render_note_bare_column_in_order` | order 含 bare 列名（如 `stage`） | SQL 用 json_extract_string，不是直接列名 |

---

## Task 6：更新文档

### 6.1 README.md

在 `### note` 小节，`note verify` 之后新增：

````markdown
**Render a note (expand .base embeds):**

```bash
markbase note render <n>            # list format (default)
markbase note render <n> -o table   # Markdown table
markbase note render <n> --dry-run  # show SQL without executing
```

Renders the note body to stdout. Each `![[*.base]]` embed is replaced with
query results from the corresponding Obsidian Base file. Non-`.base` embeds
are passed through unchanged.

Supported filters: `link(this)`, `link("name")`, `file.hasLink(this.file)`,
`file.hasTag()`, `file.inFolder()`, date comparisons, `isEmpty()`, `contains()`.

Warnings (unsupported filters, missing base files) go to stderr.
Exit code is non-zero only on hard errors (e.g. note not found).
````

### 6.2 AGENTS.md

在 `§5.1 Module Overview` 的 `verifier.rs` 行之后新增：

```
├── renderer/
│   ├── mod.rs    # note render command, .base embed expansion, Step 0-5 pipeline
│   ├── filter.rs # Base filter → DuckDB SQL translation; column/sort translation; ThisContext
│   └── output.rs # list / table output formatting; ColumnMeta definition
```

在 `§5.2 Key Design Decisions` 新增：

```
**`renderer/`**:
- Stateless pipeline: reads DB and filesystem, never writes
- filter.rs: link(this) → '[[name]]' string literal; bare column names always resolve
  to note.* (json_extract_string), never direct DB columns
- .base files indexed as non-md: name column contains full filename including extension
  (e.g. "opps.base"), query must NOT strip the extension
- db.query() called with usize::MAX limit, bypassing executor.rs default 1000
- order field = SELECT columns; sort field = ORDER BY (independent, not related to order)
```

### 6.3 CHANGELOG.md

在最新版本块中追加：

```markdown
### Added

#### Note Rendering
- **New `note render` command** — renders a note to stdout with Obsidian Base embed expansion:
  - `link(this)` and `link("name")` translated to wikilink string literals for property matching
  - `file.hasLink(this.file)`, `file.hasTag()` (with nested tag support), `file.inFolder()`,
    date arithmetic (`"30d"`, `"1 year"` formats), `isEmpty()`, `contains()` filters supported
  - `order` field maps to SELECT columns; `sort` field maps to ORDER BY (independent fields)
  - bare column names in `order`/`sort` resolve to note properties, not DB columns
  - list and table output formats; `--dry-run` for SQL inspection
  - warnings (unsupported filters, missing base files) to stderr; exit 0 on warnings only
```

---

## Task 7：Pre-commit 检查

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo build --release
```

全部通过后提交：

```bash
git add -A
git commit -m "feat: add note render command with .base embed expansion"
```

---

## 关键约束速查表

| 约束 | 具体规则 |
|---|---|
| 禁止 `.unwrap()` / `.expect()` | 非测试代码全部用 `?` 或 `.map_err()`，参照 `Rename` / `Verify` 分支写法 |
| stdout vs stderr | 渲染内容（正文 + base 结果）→ stdout；WARN / ERROR → stderr |
| exit code | note 不存在 → `Err` → main.rs `process::exit(1)`；WARN → exit 0 |
| 模块结构 | `src/renderer/mod.rs`（目录结构），**不是** `src/renderer.rs` |
| lib.rs 和 main.rs 都要改 | `lib.rs` 加 `pub mod renderer`；`main.rs` 加 `mod renderer` |
| `.base` 文件的 name 列 | 含扩展名（`"foo.base"`），查询时**不要**去掉扩展名 |
| `db.query()` 调用 | `db.query(&sql, "", usize::MAX)`，不用 executor.rs 的默认 1000 |
| `QueryResult` 类型 | `(Vec<String>, Vec<Vec<String>>)` — 列名列表 + 行数据，所有值均为 String |
| bare 列名翻译 | `order` / `sort` 中 bare 名（如 `name`、`tags`、`stage`）→ `json_extract_string`，不是直接列名 |
| `link(this)` 翻译 | → `'[[<this.name>]]'` 字面量，用于等值比较或 `list_contains` |
| `order` vs `sort` | 完全独立：`order` = SELECT 哪些列；`sort` = ORDER BY 按什么排序 |
| `views` 为空时 | 静默跳过，不输出任何内容（包括注释行） |
| 单引号转义 | 拼接 SQL 时，name 或 filter 值中的 `'` 替换为 `''` |

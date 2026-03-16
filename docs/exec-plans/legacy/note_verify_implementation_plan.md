# `note verify` 命令实现计划

**面向：** Coding Agent  
**参考设计：** `docs/design-docs/design-004-note-verify.md`  
**影响文件：** `src/main.rs`、`src/verifier.rs`（新增）、`src/lib.rs`、`tests/cli_note.rs`、`tests/common.rs`、`README.md`、`AGENTS.md`

在开始前请先阅读：`AGENTS.md`、`docs/design-docs/legacy/template_schema.md`、`docs/design-docs/design-004-note-verify.md`

---

## 总体顺序

0. 创建 feature branch
1. 新增 `src/verifier.rs`，实现核心校验逻辑
2. 修改 `src/main.rs`，注册 CLI 子命令并调用 verifier
3. 修改 `src/lib.rs`，导出 verifier 模块
4. 在 `tests/common.rs` 添加辅助方法，在 `tests/cli_note.rs` 补充端到端测试
5. 更新 `README.md` 和 `AGENTS.md`
6. 运行 pre-commit 检查，确认全绿后提交

---

## Task 0：创建 Feature Branch

```bash
git checkout -b feat/note-verify
```

不要在 `main` 分支上直接开发。

---

## Task 1：新增 `src/verifier.rs`

这是本次实现的核心文件，从上往下按以下结构编写。

### 1.1 依赖与结构体定义

```rust
use std::path::Path;
use gray_matter::Matter;
use gray_matter::engine::YAML;
use serde_json::Value;
use crate::db::Database;
```

定义以下公开结构体和枚举：

```rust
/// 单条校验问题
#[derive(Debug)]
pub struct VerifyIssue {
    pub level: IssueLevel,
    pub message: String,
}

#[derive(Debug, PartialEq)]
pub enum IssueLevel {
    Error,
    Warn,
    Info,
}

/// verify_note() 的返回值
pub struct VerifyResult {
    pub note_name: String,
    pub template_names: Vec<String>,  // 用于 summary 行显示
    pub issues: Vec<VerifyIssue>,
}

impl VerifyResult {
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.level == IssueLevel::Error)
    }
    pub fn has_warnings(&self) -> bool {
        self.issues.iter().any(|i| i.level == IssueLevel::Warn)
    }
    pub fn error_count(&self) -> usize {
        self.issues.iter().filter(|i| i.level == IssueLevel::Error).count()
    }
    pub fn warn_count(&self) -> usize {
        self.issues.iter().filter(|i| i.level == IssueLevel::Warn).count()
    }
}
```

### 1.2 主入口函数签名

```rust
pub fn verify_note(
    base_dir: &Path,
    db: &Database,
    name: &str,
) -> Result<VerifyResult, Box<dyn std::error::Error>>
```

`Result::Err` 仅用于**系统级错误**（数据库连接失败、文件 IO 异常等）。业务层 ERROR（note 未找到、template 文件不存在）一律作为 `VerifyIssue { level: Error }` 收入 `VerifyResult.issues`，函数返回 `Ok(result)`。

函数内部禁止使用 `.unwrap()` 或 `.expect()`，一律用 `?` 或 `map_err`。

### 1.3 Step 0：定位 note

执行 SQL：
```sql
SELECT folder, properties FROM notes WHERE name = ?
```

使用 `db.query()` 接口（参考 `db.rs` 中现有的 `query` 方法调用方式）。

- 结果为空 → push Error issue，提前返回 `Ok(result)`（后续步骤全部跳过）
- 结果 > 1 → push Error issue，提前返回 `Ok(result)`
- 正好 1 条 → 提取 `folder: String` 和 `properties: Value`（`serde_json::from_str`）

### 1.4 Step 1：检查 templates 字段

从 `properties` JSON 中取 `templates` 字段：

```rust
let templates_val = properties.get("templates");
```

- 不存在，或不是 `Value::Array`，或数组为空 → push Error issue，提前返回
- 遍历数组，对每个元素：
  - 期望是字符串且符合 `[[...]]` 格式（正则：`^\[\[(.+)\]\]$`）
  - 不符合 → push Error issue，提前返回

成功后收集 template name 列表（去掉 `[[` `]]`）：
```rust
let template_names: Vec<String> = ...;
```

### 1.5 Step 2：加载 template 文件，多 template 冲突检测

对每个 `template_name`，读取 `base_dir/templates/<template_name>.md`：

```rust
let tmpl_path = base_dir.join("templates").join(format!("{}.md", template_name));
```

- 文件不存在 → push Error issue，提前返回（整个命令终止）
- 用 `gray_matter::Matter::<YAML>::new().parse::<Value>()` 解析，提取 `_schema`
- 若无 `_schema`，跳过该 template 的 Step 3–5，继续下一个 template

收集所有 template 的 `_schema.properties` 后，做**冲突检测**：遍历每个字段名，若两个 template 对同一字段定义了不同 `type` → push Warn issue，冲突字段以列表中靠前的 template 为准。

冲突检测完成后，把所有 template 的校验上下文合并（字段定义以靠前者为准），统一传入后续步骤。

### 1.6 Step 3：location 校验

对每个有 `_schema.location` 的 template：

```rust
let location = schema.get("location").and_then(|v| v.as_str());
```

比较 note 的 `folder` 和 `location`，比较前统一规范化尾部斜杠（见"关键约束"节）。

- 不匹配 → push Warn issue，**继续执行后续步骤**

### 1.7 Step 4：校验模板 frontmatter 非 `_schema` 字段

提取 template frontmatter 中除 `_schema` 以外的所有字段（这些字段在 `gray_matter` 解析后以 `serde_json::Value::Object` 形式存在）。

对每个模板字段 `(key, tmpl_val)`：

**4.1 字段存在性：**
```rust
let note_val = properties.get(key);
if note_val.is_none() { push Warn }
```

**4.2 非 list 值一致性：**
- `tmpl_val` 不是 `Value::Array`
- `tmpl_val` 不是 `Value::Null` 且不是空字符串
- `note_val` 存在但与 `tmpl_val` 不相等
→ push Warn，显示期望值和实际值

**4.3 list 包含性（含 `templates` 字段本身）：**
- `tmpl_val` 是 `Value::Array`
- `note_val` 是 `Value::Array`
- 对 `tmpl_val` 中每个元素，检查 `note_val` 数组是否包含
- 缺少任意元素 → push Warn，列出 missing 元素

### 1.8 Step 5：校验 `_schema.properties`

从合并后的 schema 中取 `properties` 对象，取 `required` 列表。

**5.1 required 字段存在性：**
```rust
let required: Vec<&str> = schema.get("required")
    .and_then(|v| v.as_array())
    .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
    .unwrap_or_default();
```
对每个 required 字段：note 的对应字段为 None、`Value::Null`、空字符串、空数组 → push Warn

**5.2 类型校验：**
对 `_schema.properties` 中每个字段，若 note 中实际存在该字段：

```rust
fn check_type(val: &Value, expected_type: &str) -> bool {
    match expected_type {
        "text"     => val.is_string(),
        "number"   => val.is_number() || val.as_str().map(|s| s.parse::<f64>().is_ok()).unwrap_or(false),
        "boolean"  => val.is_boolean(),
        "date"     => val.as_str().map(|s| is_date(s)).unwrap_or(false),
        "datetime" => val.as_str().map(|s| is_datetime(s)).unwrap_or(false),
        "list"     => val.is_array(),
        _          => true,  // 未知类型不检查
    }
}
```

`is_date(s)`: 匹配 `YYYY-MM-DD`（简单正则即可，无需完整日历验证）  
`is_datetime(s)`: 匹配 `YYYY-MM-DDTHH:MM`

类型不匹配 → push Warn

**5.3 enum 校验：**
若 schema 属性定义了 `enum` 数组：
- `list` 类型：note 数组中每个元素都必须在 enum 内
- 其他类型：note 字段值必须在 enum 内
不符合 → push Warn，列出 allowed values

**5.4 link 格式与 target 校验：**
若 schema 属性定义了 `format: "link"`：

- 字段值为空（`""`、`[]`）→ 跳过（由 5.1 覆盖）
- 对 text 类型：字段值是单个字符串
- 对 list 类型：遍历数组每个元素

对每个值：
1. 若是 `[?[...]]` 格式（悬空引用） → push Info，跳过 target 检查
2. 不符合 `[[note-name]]` 格式 → push Warn
3. 提取 `note-name`，执行数据库查询确认 note 存在：
   ```sql
   SELECT properties FROM notes WHERE name = ?
   ```
   不存在 → push Warn
4. 若 schema 定义了 `target`，检查目标 note 的 `properties.type` 是否等于 target → 不符合 push Warn

wiki-link 解析请**直接复用** `extractor.rs` 中的 `WIKILINK_RE` 常量，不要重新写正则。

---

## Task 2：修改 `src/main.rs`

### 2.1 在 `NoteCommands` 枚举中添加 Verify 变体

```rust
#[derive(Subcommand)]
enum NoteCommands {
    // ... 已有的 New、Rename ...
    #[command(about = "Verify a note against its template schema")]
    Verify {
        #[arg(help = "Note name (without .md extension)")]
        name: String,
    },
}
```

### 2.2 在 `Commands::Note` 的 match 分支中添加处理

**输出规则（严格遵守 AGENTS.md § 15.2）：**
- 成功确认（passed all checks）→ `println!`（stdout）
- 所有 WARN、ERROR、INFO 信息 → `eprintln!`（stderr）
- Summary 行 → `eprintln!`（stderr）

**退出码规则（严格遵守 AGENTS.md § 15.3）：**
- 无任何问题 → exit 0（正常 `Ok(())` 返回）
- 仅有 WARN → exit 0，但 warn 信息已输出到 stderr
- 有 ERROR → exit 1（通过 `Err(...)` 返回或 `process::exit(1)`）

```rust
NoteCommands::Verify { name } => {
    let base = get_base_dir_absolute_with_cli(cli.base_dir.clone())?;
    check_db_exists(&db_path, &base_dir)?;
    let db = Mutex::new(Database::open_existing(&db_path)?);
    let db = db.lock().map_err(|e| format!("failed to acquire db lock: {e}"))?;

    let result = verifier::verify_note(&base, &db, &name)?;

    let template_list = result.template_names.join(", ");

    if result.issues.is_empty() {
        // 成功 → stdout
        println!("✓ note '{}' passed all checks against: {}.", name, template_list);
        return Ok(());
    }

    // 有问题时，header 和问题列表都走 stderr
    eprintln!("Verifying note '{}' against template(s): {}\n", name, template_list);
    for issue in &result.issues {
        let prefix = match issue.level {
            verifier::IssueLevel::Error => "[ERROR]",
            verifier::IssueLevel::Warn  => "[WARN]",
            verifier::IssueLevel::Info  => "[INFO]",
        };
        eprintln!("  {} {}", prefix, issue.message);
    }
    eprintln!();

    if result.has_errors() {
        eprintln!("Verification failed: {} error(s), {} warning(s).",
            result.error_count(), result.warn_count());
        // 以 Err 返回，main() 的错误处理统一输出并以 exit 1 退出
        return Err(format!(
            "note '{}' failed verification with {} error(s)",
            name, result.error_count()
        ).into());
    }

    // 仅有 WARN：exit 0，warn 已输出到 stderr
    eprintln!("Verification completed with issues: 0 error(s), {} warning(s).",
        result.warn_count());
}
```

> **注意：** `db.lock().unwrap()` 不能在非测试代码中使用（AGENTS.md § 13.1）。改用 `.map_err(|e| format!(...))` 或 `?`。

### 2.3 添加模块声明

参考现有的 `mod creator;`、`mod renamer;` 的写法，在 `main.rs` 顶部添加：
```rust
mod verifier;
```

---

## Task 3：修改 `src/lib.rs`

参考现有模块的导出方式，添加：
```rust
pub mod verifier;
```

---

## Task 4：测试

### 4.1 在 `tests/common.rs` 的 `TestVault` impl 中添加辅助方法

```rust
pub fn note_verify(&self, name: &str) -> Output {
    self.run_cli(&["note", "verify", name])
}
```

### 4.2 在 `tests/cli_note.rs` 中添加端到端测试

每个测试用例独立 `#[test]` 函数。断言方式参考现有测试：`assert_cli_success`、`assert_cli_error`、`stdout_contains`。

注意：按照更新后的输出规则，WARN/ERROR 信息在 **stderr**，断言时应检查 `stderr` 字段，而非 `stdout`。

**TC-1：note 不存在**
- vault 无任何 note，运行 `note verify nonexistent`
- 断言：`assert_cli_error`；stderr 包含 `not found`

**TC-2：note 无 templates 字段**
- 创建 note，frontmatter 无 `templates` 字段；index
- 断言：`assert_cli_error`；stderr 包含 `no 'templates'`

**TC-3：templates 包含非 link 格式元素**
- 创建 note，`templates: ["company_customer"]`（无 `[[]]`）；index
- 断言：`assert_cli_error`；stderr 包含 `invalid link`

**TC-4：template 文件不存在**
- 创建 note，`templates: ["[[ghost_template]]"]`；index
- 断言：`assert_cli_error`；stderr 包含 `not found`

**TC-5：location 不匹配（仅 warn，exit 0）**
- 创建 template（带 `_schema.location: company/`）；在根目录创建 note；index
- 断言：`assert_cli_success`（exit 0）；stderr 包含 `requires location`

**TC-6：required 字段缺失（仅 warn，exit 0）**
- 创建 template，`_schema.required: [industry]`；创建 note，不含 `industry` 字段；index
- 断言：`assert_cli_success`；stderr 包含 `required field 'industry'`

**TC-7：类型不匹配（仅 warn，exit 0）**
- 创建 template，properties 定义 `count: {type: number}`；创建 note，`count: "not-a-number"`；index
- 断言：`assert_cli_success`；stderr 包含 `type mismatch`

**TC-8：enum 校验失败（仅 warn，exit 0）**
- 创建 template，`size: {type: text, enum: [startup, smb, enterprise]}`；创建 note，`size: invalid`；index
- 断言：`assert_cli_success`；stderr 包含 `invalid value`；stderr 包含 `startup`

**TC-9：link target 类型不匹配（仅 warn，exit 0）**
- 创建 template，`related: {type: text, format: link, target: person}`；创建 target note（`type: company`）；创建 note，`related: "[[target-note]]"`；index
- 断言：`assert_cli_success`；stderr 包含 `requires target type 'person'`

**TC-10：全部通过**
- 创建完整合规的 template 和 note（满足所有约束）；index
- 断言：`assert_cli_success`；stdout 包含 `passed all checks`；stderr 为空

---

## Task 5：更新文档

### 5.1 更新 `README.md`

在 `note` 命令的说明章节中，补充 `verify` 子命令的用法：

```markdown
**Verify a note against its template schema:**

```bash
markbase note verify <name>
```

Checks that the note conforms to all constraints defined in its referenced MTS template(s):
- Directory location matches `_schema.location`
- Required frontmatter fields are present
- Field types and enum values are correct
- Link fields point to notes of the expected `type`

Warnings are reported to stderr. Exit code is non-zero only on errors (e.g. missing note or template file).
```

### 5.2 更新 `AGENTS.md`

**在 §5.1 模块列表中**，添加 `verifier.rs` 条目：

```
├── verifier.rs      # note verify command, MTS schema validation
```

**在 §5.2 Key Design Decisions 中**，添加 `verifier.rs` 条目：

```
**`verifier.rs`**:
- Stateless validator, reads from DB and filesystem but never writes
- Business-level errors (note not found, template missing) are returned as VerifyIssue, not Err
- Reuses WIKILINK_RE from extractor.rs for link parsing
- All output routing (stdout vs stderr) is handled by main.rs, not verifier.rs
```

**在 §11 Development Status 的 Completed ✅ 列表中**，添加：
```
- Note schema verification (note verify)
```

---

## Task 6：Pre-commit 检查

按顺序执行，全部通过后提交：

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

提交信息格式：
```
feat(note): add note verify command

Validates a note against its MTS template schema including
location, required fields, types, enums, and link targets.
Warnings output to stderr; exit 0 on warnings, exit 1 on errors.
```

---

## 关键约束与注意事项

**错误处理（AGENTS.md § 13.1）：**
- 禁止在非测试代码中使用 `.unwrap()` 或 `.expect()`
- 使用 `thiserror` 若需要定义结构化错误类型；否则用 `Box<dyn std::error::Error>` 与现有模块保持一致
- 错误信息应说明失败原因，如 `"failed to read template file '{path}': {source}"`

**输出路由（AGENTS.md § 15.2）：**
- 成功确认 → stdout（`println!`）
- WARN/ERROR/INFO/summary → stderr（`eprintln!`）

**退出码（AGENTS.md § 15.3）：**
- 退出码 `2` 在本项目中**不使用**：仅有 WARN 时退出码为 `0`
- ERROR 通过 `return Err(...)` 实现退出码 `1`，不用 `process::exit`

**不要重复造轮子：**
- wiki-link 解析：复用 `extractor.rs` 中的 `WIKILINK_RE`
- frontmatter 解析：复用 `gray_matter::Matter::<YAML>::new().parse::<Value>()`，与 `creator.rs` 做法一致
- 数据库查询：复用 `db.query()`，参考现有调用方式

**`folder` 字段格式：** 数据库中 `folder` 列存储相对路径（如 `company/`，根目录 note 为空字符串）。比较前先规范化两者的尾部斜杠，避免 `company` vs `company/` 的比较失败。

**`properties` JSON 的 list 字段：** YAML 中的 `["[[a]]", "[[b]]"]` 经 gray_matter 解析、数据库存储、再 `serde_json::from_str` 还原后均为 `Value::Array`。枚举元素比较时注意 `Value::String` 的内容不含外层引号。

**多 template 场景下的 `template_names`：** `VerifyResult.template_names` 只收录**实际存在且含 `_schema` 的** template 名；若某 template 文件不存在则提前 ERROR 返回，不进入 `template_names`。

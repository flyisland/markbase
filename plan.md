# Agent 工作流支持 - 实现计划

## 目标

为 mdb 添加 Agent 工作流支持，使每一步的输出可直接作为下一步的输入，实现自洽的自动化流程：

```
list templates → describe template X → create note --template X → 完善 note
```

## 现状分析

| 需求 | 当前实现 | 差距 |
|------|---------|------|
| `mdb template list` | ✅ 已实现 | 需适配 JSON 输出 |
| `mdb template describe X` | ❌ 未实现 | 需新增命令 |
| `mdb new X --template Y` | ⚠️ 只输出路径 | 需返回完整内容 |
| `MDB_OUTPUT` 环境变量 | ❌ 未实现 | 需添加 |

### 当前命令结构

| 命令 | `-o` 参数支持 |
|------|--------------|
| `mdb index` | ❌ |
| `mdb query` | ✅ |
| `mdb new` | ❌ |
| `mdb template list` | ❌ |

---

## 设计方案

### 1. 命令结构

保持现有结构，添加 `describe` 子命令：

```bash
mdb template list                   # 列出模板
mdb template describe <name>        # 查看模板内容（新增）
```

### 2. 环境变量 `MDB_OUTPUT`

```bash
export MDB_OUTPUT=json
```

**位置**: `src/main.rs`

```rust
const ENV_OUTPUT: &str = "MDB_OUTPUT";

#[derive(Parser)]
struct Cli {
    // ... existing fields
    
    #[arg(
        long = "output-format",
        short = 'o',
        global = true,
        env = ENV_OUTPUT,
        help_heading = "Output",
        help = "Output format: table, json, list"
    )]
    output_format: Option<OutputFormat>,
}
```

**影响范围**:
- `query` 命令（原来就有 `-o` 参数）
- `template list` 命令（新增）

**优先级**: CLI `-o` > `MDB_OUTPUT` 环境变量 > 默认值 (`table`)

### 3. 各命令详细规格

#### 3.1 `mdb query`

**功能**: 查询已索引的文件

**输入**:
```bash
mdb query <QUERY> [OPTIONS]
```

**参数**:
| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `QUERY` | string | 必填 | 查询表达式 |
| `-o`, `--output-format` | string | `table` | 输出格式: `table`, `json`, `list` |
| `-f`, `--output-fields` | string | `path,mtime` | 输出字段 |
| `-l`, `--limit` | integer | `1000` | 限制结果数量 |

**环境变量**: `MDB_OUTPUT` 影响默认格式

---

#### 3.2 `mdb template list`

**功能**: 列出所有可用模板

**输入**:
```bash
mdb template list [OPTIONS]
```

**参数**:
| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `-o`, `--output-format` | string | `table` | 输出格式: `table`, `json`, `list` |
| `-f`, `--output-fields` | string | `name,_schema.description,path` | 额外显示字段 |

**环境变量**: `MDB_OUTPUT` 影响默认格式

**输出示例**:

**table 格式** (默认):
```
name      description    path
─────────────────────────────────────
daily     日报模板       ./templates/daily.md
project   项目模板       ./templates/project.md

2 results
```

**json 格式**:
```json
{
  "metadata": { "count": 2 },
  "results": [
    { "name": "daily", "path": "./templates/daily.md", "description": "日报模板" },
    { "name": "project", "path": "./templates/project.md", "description": "项目模板" }
  ]
}
```

**list 格式**:
```
name: daily
description: 日报模板
path: ./templates/daily.md
---
name: project
description: 项目模板
path: ./templates/project.md

2 results
```

---

#### 3.3 `mdb template describe <name>`

**功能**: 查看指定模板的原始内容

**输入**:
```bash
mdb template describe <NAME>
```

**参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `NAME` | string | 是 | 模板名称（不含 .md 后缀） |

**环境变量**: 不受影响

**输出**: 原始 markdown 内容，无任何装饰

**成功示例**:
```bash
$ mdb template describe daily
---
date: ""          # 日期，格式 YYYY-MM-DD，必填
mood: ""          # 今日心情，选填
summary: ""       # 今日总结，必填
tags: []
---

## 今日记录

（正文内容说明...）
```

**错误示例**:
```
Error: Template 'nonexistent' not found
```

---

#### 3.3 `mdb new <name> --template <tmpl>`

**功能**: 使用模板创建新笔记

**输入**:
```bash
mdb new <NAME> --template <TEMPLATE>
```

**参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `NAME` | string | 是 | 笔记名称（不含 .md 后缀） |
| `--template`, `-t` | string | 否 | 模板名称（不含 .md 后缀） |

**注意**: 
- 不使用 `--template` 时，输出路径文本
- 使用 `--template` 时，**固定输出 list 格式**（不需要 `-o` 参数）

**环境变量**: 不受影响

**输出示例**:

**有模板** (固定 list 格式):
```bash
$ mdb new today --template daily
path: ./notes/today.md
content: ---
date: ""
mood: ""
summary: ""
tags: []
---

## 今日记录

---
```

**无模板** (保持原行为):
```
$ mdb new my-note
Created: ./notes/my-note.md
```

**错误示例**:
```
Error: Template 'nonexistent' not found
```

```
Error: Note 'my-note' already exists
```

---

## 核心代码改动

### 1. src/describe.rs (新增)

```rust
use std::fs;
use std::path::Path;

pub fn describe_template(
    base_dir: &Path,
    name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let template_path = base_dir.join("templates").join(format!("{}.md", name));
    if !template_path.exists() {
        return Err(format!("Template '{}' not found", name).into());
    }
    fs::read_to_string(&template_path)
}
```

### 2. src/creator.rs (修改)

新增 `CreatedNote` 结构体，修改 `create_note` 返回值：

```rust
use std::path::PathBuf;

pub struct CreatedNote {
    pub path: PathBuf,
    pub content: String,
}

pub fn create_note(
    base_dir: &Path,
    name: &str,
    template_name: Option<&str>,
) -> Result<CreatedNote, Box<dyn std::error::Error>> {
    // ... 现有逻辑
    // 返回 CreatedNote { path, content }
}
```

### 3. src/main.rs (修改)

#### 3.1 添加环境变量支持

```rust
const ENV_OUTPUT: &str = "MDB_OUTPUT";

#[derive(Parser)]
struct Cli {
    // ... existing fields
    
    #[arg(
        long = "output-format",
        short = 'o',
        global = true,
        env = ENV_OUTPUT,
        help_heading = "Output",
        help = "Output format: table, json, list"
    )]
    output_format: Option<OutputFormat>,
}
```

#### 3.2 添加 TemplateCommands::Describe

```rust
#[derive(Subcommand)]
enum TemplateCommands {
    List { ... },
    Describe { name: String },
}
```

#### 3.3 query 命令使用 MDB_OUTPUT

```rust
Commands::Query { query, format, limit, fields } => {
    let effective_format = format.unwrap_or_else(|| {
        env::var(ENV_OUTPUT)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(OutputFormat::Table)
    });
    // ... 
}
```

#### 3.4 template list 支持 -o 参数

```rust
TemplateCommands::List { fields } => {
    let effective_format = cli.output_format.unwrap_or_else(|| {
        env::var(ENV_OUTPUT)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(OutputFormat::Table)
    });
    // 使用 effective_format 而非硬编码
}
```

#### 3.5 new --template 输出 list 格式

```rust
Commands::New { name, template } => {
    let base = cli.base_dir.unwrap_or_else(get_base_dir);
    
    if let Some(tmpl) = template {
        let created = creator::create_note(&base, &name, Some(&tmpl))?;
        // 输出 list 格式
        println!("path: {}", created.path.display());
        println!("content: {}", created.content);
    } else {
        let path = creator::create_note(&base, &name, None)?;
        println!("Created: {}", path.display());
    }
}
```

---

## 文件变更清单

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `src/main.rs` | 修改 | 添加 env 变量、describe 命令、new --template list 输出 |
| `src/describe.rs` | **新增** | 模板描述功能 |
| `src/creator.rs` | 修改 | 返回 `CreatedNote` 包含 content |
| `src/lib.rs` | 修改 | 导出 describe 模块 |
| `README.md` | 修改 | 更新命令文档 |
| `AGENTS.md` | 修改 | 更新开发状态 |

---

## 开发任务拆分

### 任务 1: `mdb template describe` 命令

**目标**: 实现查看模板内容的功能

**文件变更**:
| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `src/describe.rs` | 新增 | 模板描述功能 |
| `src/lib.rs` | 修改 | 导出 describe 模块 |
| `src/main.rs` | 修改 | 添加 `TemplateCommands::Describe` |
| `README.md` | 修改 | 更新命令文档 |
| `AGENTS.md` | 修改 | 更新开发状态 |

**实现步骤**:
1. 创建 `src/describe.rs`，实现 `describe_template` 函数
2. 修改 `src/lib.rs` 导出 describe 模块
3. 修改 `src/main.rs`:
   - 添加 `TemplateCommands::Describe` 变体
   - 实现 `template describe` 命令处理
4. 添加单元测试
5. 运行 `cargo clippy` 和 `cargo test`
6. 更新 README.md 和 AGENTS.md

**验证**:
```bash
$ mdb template describe daily
---
date: ""          # 日期，格式 YYYY-MM-DD，必填
...
```

---

### 任务 2: 环境变量 `MDB_OUTPUT` + `template list -o`

**目标**: 统一输出格式控制，支持环境变量配置

**文件变更**:
| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `src/main.rs` | 修改 | 添加全局 --output-format 参数，支持 env 变量 |
| `README.md` | 修改 | 更新命令文档和环境变量说明 |
| `AGENTS.md` | 修改 | 更新开发状态 |

**实现步骤**:
1. 修改 `src/main.rs`:
   - 添加 `ENV_OUTPUT` 常量
   - 添加 `--output-format` / `-o` 全局参数到 `Cli`
   - 修改 `query` 命令：优先使用 CLI 参数，其次 env 变量，默认 `table`
   - 修改 `template list` 命令：支持 `-o` 参数，受 `MDB_OUTPUT` 影响
2. 添加单元测试
3. 运行 `cargo clippy` 和 `cargo test`
4. 更新 README.md 和 AGENTS.md

**验证**:
```bash
# CLI 参数优先
$ mdb query "name == 'test'" -o json

# 环境变量
$ export MDB_OUTPUT=json
$ mdb query "name == 'test'"

# template list 支持
$ mdb template list -o json
```

---

### 任务 3: `new --template` 输出改进

**目标**: 使用模板创建笔记时，返回完整内容供 agent 使用

**文件变更**:
| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `src/creator.rs` | 修改 | 返回 `CreatedNote` 包含 content |
| `src/main.rs` | 修改 | 输出 list 格式 |
| `README.md` | 修改 | 更新命令文档 |
| `AGENTS.md` | 修改 | 更新开发状态 |

**实现步骤**:
1. 修改 `src/creator.rs`:
   - 新增 `CreatedNote` 结构体
   - 修改 `create_note` 返回 `CreatedNote`
   - 保持 API 兼容（内部逻辑不变）
2. 修改 `src/main.rs`:
   - 当使用 `--template` 时，输出 list 格式（path + content）
   - 不使用 `--template` 时，保持原行为
3. 添加单元测试
4. 运行 `cargo clippy` 和 `cargo test`
5. 更新 README.md 和 AGENTS.md

**验证**:
```bash
$ mdb new today --template daily
path: ./notes/today.md
content: ---
date: ""
mood: ""
...

$ mdb new my-note
Created: ./notes/my-note.md
```

---

## 开发顺序

建议按任务顺序 1 → 2 → 3 开发，每个任务独立可测试。

---

## 测试用例

```rust
// 任务 1: describe
#[test]
fn test_describe_template_success() { ... }
#[test]
fn test_describe_template_not_found() { ... }

// 任务 2: MDB_OUTPUT
#[test]
fn test_mdb_output_env_variable() { ... }
#[test]
fn test_template_list_output_format() { ... }

// 任务 3: new --template
#[test]
fn test_create_note_returns_content() { ... }
#[test]
fn test_new_with_template_list_output() { ... }
```

---

## Agent 工作流示例

```bash
# 1. 列出可用模板
$ mdb template list -o json
{
  "metadata": { "count": 2 },
  "results": [
    { "name": "daily", "path": "./templates/daily.md", "description": "日报模板" },
    { "name": "project", "path": "./templates/project.md", "description": "项目模板" }
  ]
}

# 2. 查看模板内容
$ mdb template describe daily
---
date: ""          # 日期，格式 YYYY-MM-DD，必填
mood: ""          # 今日心情，选填
summary: ""       # 今日总结，必填
tags: []
---

## 今日记录

（正文内容说明...）

# 3. 创建笔记（使用模板）
$ mdb new today --template daily
path: ./notes/today.md
content: ---
date: ""
mood: ""
summary: ""
tags: []
---

## 今日记录

---

# 4. 后续步骤：Agent 填充内容后写入文件
```

---

## 环境变量汇总

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `MDB_DATABASE` | 数据库路径 | `.mdb/mdb.duckdb` |
| `MDB_BASE_DIR` | 基础目录 | `.` |
| `MDB_OUTPUT` | 输出格式 | `table` |

**优先级**: CLI 参数 > 环境变量 > 默认值

# Markbase 开发指南

本文档面向 markbase 的开发者和开发 Agent，包含架构设计、开发规范和实现细节。

## 1. 项目概述

Markbase 是一个高性能 CLI 工具，用于扫描、解析 Markdown 笔记并索引到 DuckDB 数据库，支持即时元数据查询，兼容 Obsidian。

**核心价值**：为 AI Agent 提供结构化的 Markdown 知识库访问能力。

## 2. 技术栈

- **语言**: Rust 1.85+ (2024 edition)
- **CLI 框架**: clap v4.5 (derive feature)
- **数据库**: DuckDB (bundled with `duckdb` crate)
- **文件遍历**: walkdir v2.5
- **解析**: gray_matter (frontmatter), regex (wiki-links/tags)
- **序列化**: serde, serde_json

**设计原则**: 最小化依赖，优化二进制体积 (`strip = true` in Cargo.toml)

## 3. 数据模型

### 3.1 Schema

数据库位置: `{{base-dir}}/.markbase/markbase.duckdb`

```sql
CREATE TABLE notes (
    path       TEXT PRIMARY KEY,        -- 相对 base-dir 的路径
    folder     TEXT,                    -- 目录路径
    name       TEXT,                    -- 文件名(无扩展名)
    ext        TEXT,                    -- 扩展名
    size       INTEGER,                 -- 字节数
    ctime      TIMESTAMPTZ,             -- 创建时间
    mtime      TIMESTAMPTZ,             -- 修改时间
    tags       VARCHAR[],               -- 标签数组
    links      VARCHAR[],               -- wiki-links 数组
    backlinks  VARCHAR[],               -- 反向链接数组
    embeds     VARCHAR[],               -- 嵌入数组
    properties JSON                     -- frontmatter 属性
);
```

**索引**:
```sql
CREATE INDEX idx_mtime ON notes(mtime);
CREATE INDEX idx_folder ON notes(folder);
CREATE INDEX idx_name ON notes(name);
```

### 3.2 字段解析优先级

查询字段时:
1. 先检查保留字段 (`path`, `name`, `mtime` 等)
2. 未匹配则从 `properties` JSON 中提取: `json_extract_string(properties, '$."field"')`
3. 支持嵌套路径: `a.b.c` → `json_extract_string(properties, '$."a"."b"."c"')`

## 4. 核心模块职责

### 4.1 模块概览

```
src/
├── main.rs          # CLI 入口，参数解析与命令分发
├── db.rs            # DuckDB 连接管理、Schema 初始化、CRUD
├── scanner.rs       # index 命令驱动，目录遍历、增量更新、反向链接计算
├── extractor.rs     # 单文件解析：frontmatter、wiki-links、tags
├── creator.rs       # note new 命令，模板渲染
├── renamer.rs       # note rename 命令，链接更新
├── describe.rs      # template describe 命令
├── lib.rs           # 库导出
└── query/
    ├── mod.rs       # 输出格式化 (table/json/list)
    ├── detector.rs  # SQL/表达式模式检测、安全验证
    ├── translator.rs # 字段名翻译
    ├── error_map.rs # DuckDB 错误映射
    └── executor.rs  # 查询执行编排
```

### 4.2 关键设计决策

**`scanner.rs`**:
- 使用 WalkDir 迭代器顺序处理（避免内存峰值）
- 增量更新逻辑: 比较 `mtime + size`，跳过未修改文件
- 删除检测: 遍历后对比 DB 与文件系统，移除已删除条目
- 冲突处理: 同名文件跳过并警告
- 反向链接: 所有 note 插入后，执行二次遍历计算 backlinks

**`extractor.rs`**:
- 无状态解析器，不感知数据库
- 合并内容标签 (`#tag`) 和 frontmatter 标签
- 返回 `Note` struct，供 scanner 消费

**`db.rs`**:
- 拥有 DuckDB 连接，实现 Drop trait 确保关闭
- 使用 `INSERT OR REPLACE` 实现 upsert
- 行值访问通过 `duckdb::types::Value`，注意列顺序必须与 schema 一致

**`query/detector.rs`**:
- 模式判断: 以 `SELECT` 开头 → SQL 模式，否则 → 表达式模式
- 安全验证: 拒绝非 SELECT 语句、多语句注入

**`query/translator.rs`**:
- 保留字段直接透传
- Frontmatter 字段翻译为 `json_extract_string(properties, ...)`
- 保留类型转换语法 (`::INTEGER`, `::TIMESTAMP`)

## 5. 命令内部逻辑

详细用法见 README.md，本节说明实现细节。

### `index`

**流程**:
1. 遍历目录树 (WalkDir)
2. 对每个 `.md` 文件:
   - 比较 DB 中的 `mtime + size`，相同则跳过
   - 调用 `extractor.rs` 解析内容
   - 插入/更新 DB
3. 删除检测: DB 中存在但文件系统不存在的条目 → 删除
4. 冲突检测: 同名文件 → 警告并跳过
5. 反向链接计算: 遍历所有 note 的 `links`，填充目标 note 的 `backlinks`
6. 提交事务

**`--force` 标志**: 删除 `.markbase/markbase.duckdb` 后重新索引

### `query`

**两种输入模式**:
- **表达式模式**: `author == 'Tom'` → `SELECT * FROM notes WHERE author == 'Tom'`
- **SQL 模式**: `SELECT path FROM notes WHERE ...` → 直接执行（仅翻译字段名）

**字段翻译**:
```sql
-- 表达式: author == 'John'
-- 翻译为:
SELECT * FROM notes WHERE json_extract_string(properties, '$."author"') = 'John'

-- SQL: SELECT name, author FROM notes WHERE author = 'John'
-- 翻译为:
SELECT name, json_extract_string(properties, '$."author"') FROM notes WHERE json_extract_string(properties, '$."author"') = 'John'
```

**特殊处理**:
- `list_contains(field, value)` 对 frontmatter 数组字段使用 `(properties->'$."field"')::VARCHAR[]`

**错误映射**: DuckDB 错误转换为用户友好消息（未知字段、类型转换失败等）

### `note rename`

**流程**:
1. 按 name 查找 note（非 path）
2. 唯一性检查: 同名文件存在 → 失败
3. 重命名文件
4. 遍历所有 `.md` 文件，更新 `[[old-name]]` → `[[new-name]]`
5. 保留别名和锚点: `[[old-name|alias]]` → `[[new-name|alias]]`
6. 重新索引受影响的 note

## 6. 性能目标

| 指标 | 目标 |
|------|------|
| 冷启动索引 10,000 notes | < 5s |
| 复杂查询延迟 (10k rows) | < 50ms |
| 复杂表达式编译 | < 100ms |

**优化手段**:
- 增量更新避免全量扫描
- 索引优化 (mtime, folder, name)
- 二进制体积优化 (`strip = true`)

## 7. 约束与安全

- **单写入者**: DuckDB 约束，同一时刻只能有一个 `index` 进程
- **查询安全**: 仅允许 SELECT 语句，拒绝多语句注入
- **错误处理**: 全面使用 `Result` 和 `?` 操作符
- **线程安全**: 多线程场景使用 `Mutex<Database>`
- **优雅关闭**: `Database` 实现 Drop trait

## 8. 开发命令

```bash
# 开发构建
cargo build

# 运行
export MARKBASE_BASE_DIR=./notes
cargo run -- index
cargo run -- query "name == 'readme'"

# 测试
cargo test
cargo test -- --nocapture

# 发布构建
cargo build --release

# 代码检查
cargo clippy -- -D warnings
cargo fmt --check
```

## 9. 项目结构

```
markbase/
├── Cargo.toml           # 依赖配置
├── Cargo.lock           # 依赖锁定
├── README.md            # 用户文档
├── AGENTS.md            # 本文件
├── src/
│   ├── main.rs          # CLI 入口
│   ├── db.rs            # 数据库操作
│   ├── scanner.rs       # 索引扫描
│   ├── extractor.rs     # 内容提取
│   ├── creator.rs       # Note 创建
│   ├── renamer.rs       # Note 重命名
│   ├── describe.rs      # 模板描述
│   ├── lib.rs           # 库导出
│   └── query/           # 查询系统
│       ├── mod.rs       # 输出格式化
│       ├── detector.rs  # 模式检测
│       ├── translator.rs # 字段翻译
│       ├── error_map.rs # 错误映射
│       └── executor.rs  # 查询执行
└── target/              # 构建输出
```

## 10. 开发状态

### 已完成 ✅

- 核心索引功能
- 查询系统 (SQL 模式 + 表达式模式)
- 字段翻译与安全验证
- 多输出格式 (table/json/list)
- 反向链接追踪
- 增量更新与删除检测
- Note 创建 (模板支持)
- Note 重命名 (链接更新)
- Template 管理

### 技术债务

- 集成测试覆盖
- 性能基准测试 (10k notes 目标)
- 并行索引处理
- 配置文件支持
- 查询结果缓存

### 测试覆盖

| 模块 | 覆盖范围 |
|------|----------|
| `detector.rs` | 模式检测、表达式拆分、安全验证 |
| `translator.rs` | 字段翻译、保留字段、嵌套路径、类型转换、数组处理 |
| `error_map.rs` | DuckDB 错误映射 |
| `executor.rs` | 查询执行、错误包装 |
| `extractor.rs` | Frontmatter、标签、链接、嵌入解析 |
| `db.rs` | CRUD 操作、查询 |
| `scanner.rs` | 扫描、索引、反向链接、删除检测、冲突处理 |
| `query/mod.rs` | 输出格式化 |
| `creator.rs` | 模板解析、Note 创建 |
| `renamer.rs` | 链接更新、重命名 |
| `main.rs` | CLI 参数解析 |

## 11. 开发工作流

### 11.1 分支策略

**禁止直接在 `main` 分支开发**

创建功能分支:
```bash
git checkout -b feat/<description>
git checkout -b fix/<description>
```

分支前缀: `feat/`, `fix/`, `refactor/`, `test/`, `docs/`

### 11.2 提交前检查

**必须通过以下检查**:

```bash
cargo clippy -- -D warnings  # Lint
cargo test                   # 测试
cargo fmt --check            # 格式
```

任意一项失败不得提交。

### 11.3 文档同步

**README.md 更新时机**:
- 命令、选项或行为变更
- 查询操作符或函数变更
- 环境变量变更

**AGENTS.md 更新时机**:
- 依赖或技术栈变更
- 数据模型变更
- 架构或算法变更
- 性能目标或约束变更
- 开发状态变更

### 11.4 提交信息规范

```
<type>(<scope>): <summary>

# 示例
feat(query): add exists() function support
fix(scanner): handle symlink cycles in walkdir
refactor(db): simplify upsert_note params
test(compiler): add coverage for nested JSON paths
docs(readme): update query operator table
```

### 11.5 定义完成标准

任务完成必须满足:

- [ ] 分支已同步 main
- [ ] `cargo clippy -- -D warnings` 通过
- [ ] `cargo test` 全部通过
- [ ] `cargo fmt --check` 通过
- [ ] 用户可见行为变更已更新 README.md
- [ ] 架构变更已更新 AGENTS.md

## 12. 测试策略

**测试价值**: 验证有意义的行为，而非仅仅覆盖代码路径

**必须编写测试**:
- 新功能 → 核心行为 + 关键边界
- Bug 修复 → 回归测试
- 公共接口变更 → 审查现有测试

**优秀测试的特征**:
- 验证正确输出或副作用
- 覆盖边界条件和错误路径
- 能被错误实现打破（naive 实现无法通过）

**避免**:
- 仅为覆盖率而写
- 重复现有测试
- 对无关重构过度敏感

## 13. Rust 最佳实践

### 13.1 错误处理

- 使用 `thiserror` 定义结构化错误类型
- 非测试代码禁止 `.unwrap()` / `.expect()`
- 错误消息需说明失败原因: `"failed to open {path}: {source}"` 而非 `"io error"`

### 13.2 依赖管理

- 添加新依赖前检查现有依赖是否满足需求
- 使用 `cargo add <crate>` 而非手动编辑 `Cargo.toml`
- 需要特性时使用 `cargo add <crate> --features <feature>`

### 13.3 代码风格

- 函数保持短小聚焦
- 需要注释解释的代码块考虑提取
- 正确性优先，性能优化需有测量依据

## 14. 代码复用

**模块边界尊重**:
- 每个模块有明确职责
- 不为避免重构而将逻辑放入错误模块
- 边界需要移动时显式移动

**新增功能前检查**:
- 逻辑是否已存在？先搜索
- 是否属于现有模块？
- 新模块是否有单一清晰的职责？

**新 CLI 子命令**:
- 遵循 `main.rs` 现有模式
- 逻辑实现在独立文件，不在 `main.rs` 内联

## 15. CLI UX 原则

### 15.1 输出结构

- 默认输出提供结构化摘要（计数、状态、警告）
- `--verbose` 用于过程细节
- 禁止无信息量的确认（如 "Done!"）

### 15.2 输出目标

- 查询结果和结构化数据 → stdout (支持管道)
- 警告、错误、诊断信息 → stderr

### 15.3 退出码

- 成功 → `0`
- 错误（影响预期效果）→ 非零
- 非致命警告 → `0`（但需报告到 stderr）

### 15.4 输出示例

```
# index
Indexing ./notes...
  ✓ 142 files indexed  (3 new, 5 updated, 0 errors)  [1.2s]
  ⚠ Skipped: notes/broken.md — invalid frontmatter (line 4)

# query
path                      mtime
────────────────────────  ───────────────────
./notes/task-a.md         2025-01-10 09:00:00
./notes/task-b.md         2025-01-12 14:30:00

2 results
```

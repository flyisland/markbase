# Query 命令重设计说明

## 1. 语言规范

应用的所有帮助信息（clap 生成的 `--help` 输出）和错误提示信息，统一使用英文。

---

## 2. 目标

将 `query` 命令升级为支持原生 DuckDB SQL，同时保持用户友好性。具体目标：

- 用户可以直接写原生 DuckDB SQL，获得完整的查询表达能力
- 用户无需了解底层 schema 细节（frontmatter 存在 JSON 列里）
- 舍弃原有的自定义 `has()`、`exists()` 函数，统一使用 DuckDB 原生函数
- 提供清晰易懂的错误提示

---

## 2. 命令设计

### 主命令

```bash
markbase query [OPTIONS] [SQL]
```

| 参数 | 说明 |
|---|---|
| `[SQL]` | 查询输入，可选。不传时返回所有笔记 |
| `-o table/json/list` | 输出格式，默认 `table` |

```bash
markbase query                                        # 返回所有笔记
markbase query "author == 'Tom'"                      # 表达式模式
markbase query "author == 'Tom' ORDER BY mtime DESC"  # 表达式模式 + 后续子句
markbase query -o json "SELECT path, name FROM notes WHERE list_contains(tags, 'todo')"  # SQL 模式
```

clap 实现：`[SQL]` 定义为 `Option<String>`，`None` 时补全为完整的 `SELECT ... FROM notes`。

### translate 子命令

显示翻译后实际提交给 DuckDB 的 SQL，不执行查询，用于调试。

```bash
markbase query translate [SQL]
```

```bash
markbase query translate "author == 'Tom' ORDER BY mtime DESC"
```

输出为纯文本 SQL，固定格式，无 `-o` 参数。同样支持不传 `[SQL]`，此时输出补全后的完整 SELECT 语句。

---

## 3. 两种输入模式

翻译层首先判断用户输入属于哪种模式：

**SQL 模式**：输入以 `SELECT`（大小写不敏感）开头，视为完整 SQL，翻译层只做字段名替换，不补全结构。用户需要写完整的 `FROM notes`。

**表达式模式**：输入不以 `SELECT` 开头，视为完整 SQL 去掉 `SELECT ... FROM notes` 前缀后的剩余部分，翻译层自动补全前缀。

表达式模式支持以下四种情况：

**情况一：只有 WHERE 条件**
```
author == 'Tom'
→ SELECT path, name, mtime, size, tags FROM notes WHERE author == 'Tom'
```

**情况二：WHERE 条件 + 后续子句**
```
author == 'Tom' ORDER BY mtime DESC LIMIT 10
→ SELECT path, name, mtime, size, tags FROM notes WHERE author == 'Tom' ORDER BY mtime DESC LIMIT 10
```

**情况三：只有后续子句，无 WHERE 条件**
```
ORDER BY mtime DESC LIMIT 10
→ SELECT path, name, mtime, size, tags FROM notes ORDER BY mtime DESC LIMIT 10
```

**情况四：空输入（未传 SQL 参数）**
```
（无输入）
→ SELECT path, name, mtime, size, tags FROM notes
```

识别逻辑：找用户输入中第一个顶层的 `ORDER BY`、`LIMIT`、`GROUP BY`、`HAVING` 关键字的位置：

- 输入以这些关键字之一开头 → 情况三，直接透传，不补全 `WHERE`
- 这些关键字出现在中间 → 情况二，之前的部分作为 `WHERE` 条件，之后透传
- 这些关键字都不出现 → 情况一，全部作为 `WHERE` 条件
- 输入为空 → 情况四，只补全 `SELECT ... FROM notes`

**注意**：子查询或函数参数内部出现的这些关键字不应触发以上判断（顶层识别）。当前版本暂不处理含子查询的表达式，遇到此类输入建议用户切换到 SQL 模式。

---

## 4. 安全限制

`query` 命令只允许 `SELECT` 语句，拒绝一切写操作。detector 阶段做白名单校验：

- 非 `SELECT` 开头的 SQL 模式输入（`INSERT`、`UPDATE`、`DELETE`、`DROP` 等）一律拒绝，提示 `Error: query command only supports SELECT statements`
- 拦截 `;` 分号拼接多语句的情况，提示 `Error: multiple statements are not allowed`
- 表达式模式由翻译层拼接，天然安全

---

## 5. 字段分类

数据库表名为 `notes`，schema 如下：

```sql
CREATE TABLE IF NOT EXISTS notes (
    path       TEXT PRIMARY KEY,
    folder     TEXT NOT NULL,
    name       TEXT NOT NULL,
    ext        TEXT NOT NULL,
    size       INTEGER NOT NULL,
    ctime      TIMESTAMPTZ NOT NULL,
    mtime      TIMESTAMPTZ NOT NULL,
    tags       VARCHAR[],
    links      VARCHAR[],
    backlinks  VARCHAR[],
    embeds     VARCHAR[],
    properties JSON
)
```

字段分为两类：

**保留字段**（原生列，直接透传）：

`path`, `folder`, `name`, `ext`, `size`, `ctime`, `mtime`, `tags`, `links`, `backlinks`, `embeds`

**frontmatter 字段**（存储在 `properties` JSON 列中）：

所有不在保留字段列表中的标识符，均视为 frontmatter 字段，需要翻译。

注意：`content` 字段已从 schema 中移除，agent 需要读取文件内容时应直接读取文件本身。

---

## 6. 翻译规则

翻译层扫描 SQL 文本中的所有标识符，识别出非保留字段后，根据其所在的语法位置决定翻译方式。

### 6.1 通用位置（比较、ORDER BY、SELECT 列表等）

```
author  →  json_extract_string(properties, '$."author"')
```

嵌套属性（点号分隔）：

```
_schema.strict  →  json_extract_string(properties, '$."_schema"."strict"')
```

### 6.2 出现在 `list_contains` 的第一个参数位置

```
list_contains(categories, 'work')
→
list_contains((properties->'$."categories"')::VARCHAR[], 'work')
```

保留字段（如 `tags`）在 `list_contains` 中直接透传：

```
list_contains(tags, 'todo')  →  list_contains(tags, 'todo')  （不变）
```

### 6.3 `IS NOT NULL` / `IS NULL`

```
author IS NOT NULL
→
json_extract_string(properties, '$."author"') IS NOT NULL
```

保留字段直接透传。

---

## 7. 类型处理

`json_extract_string` 始终返回 `TEXT` 类型。当用户需要对 frontmatter 字段做非字符串比较时，由用户自行使用 DuckDB 的 `::TYPE` cast 语法：

```sql
-- 用户写
year::INTEGER >= 2024

-- 翻译后
json_extract_string(properties, '$."year"')::INTEGER >= 2024
```

翻译层识别出 `year` 是非保留字段后，替换字段名部分，`::INTEGER` 修饰符作为独立的词法单元直接透传，不受影响。

**翻译层不做类型推断，不自动插入 CAST。** 类型转换完全由用户显式控制。

**关于 `list_contains` 的数字数组**：frontmatter 数组字段经过 `::VARCHAR[]` cast 后，元素均为字符串。对数字数组使用 `list_contains` 时，比较值也应写成字符串：

```sql
list_contains(years, '2024')   -- 正确
list_contains(years, 2024)     -- 类型不匹配，DuckDB 会报错
```

实践中 frontmatter 数字数组极为罕见，通常无需关注此细节。

---

## 8. 舍弃自定义函数

原有的 `has()` 和 `exists()` 函数**全部移除**，对应替换如下：

| 原写法 | 新写法 |
|---|---|
| `has(tags, 'todo')` | `list_contains(tags, 'todo')` |
| `has(links, 'page')` | `list_contains(links, 'page')` |
| `has(categories, 'work')` | `list_contains(categories, 'work')` |
| `exists(author)` | `author IS NOT NULL` |
| `exists(tags)` | `tags IS NOT NULL` |

---

## 9. 错误处理

错误来源分两类：

**翻译层预检错误**：在执行前发现的问题，直接给出友好提示。例如对非数组类型的保留字段使用 `list_contains`（翻译层可根据保留字段类型表提前检测）。

**DuckDB 执行错误**：捕获 DuckDB 返回的错误，通过一个薄的错误翻译层转成用户可读的提示。重点覆盖以下常见场景：

- 类型转换失败（`Conversion Error`）→ `Error: cannot convert value '...' for field '...', expected type is ..., e.g. use year::INTEGER >= 2024`
- 列不存在（`Column not found`）→ `Error: unknown field '...', if this is a frontmatter field check for typos`
- JSON 路径无效 → `Error: invalid nested property path '...', check the syntax e.g. _schema.strict`

错误信息应包含：**发生了什么、可能的原因、建议的修正写法**。

---

## 10. 模块建议

在现有 `query/` 目录下调整为以下结构：

```
query/
├── mod.rs          # 输出格式（table/json/list），保持不变
├── detector.rs     # 识别 SQL 模式 vs 表达式模式，以及安全校验
├── translator.rs   # 核心翻译逻辑：字段名识别与替换
├── executor.rs     # 执行翻译后的 SQL，捕获并翻译 DuckDB 错误
└── error_map.rs    # DuckDB 错误码/消息 → 用户友好提示的映射规则
```

原有的 `tokenizer.rs`、`parser.rs`、`compiler.rs` 可以移除或保留作参考，翻译层不再需要完整的 AST 解析，只需词法级别的标识符扫描与替换。

---

## 11. 完整示例

### 无输入，返回所有笔记

```
输入:  （无）

翻译:  SELECT path, name, mtime, size, tags FROM notes
```

### 表达式模式 — 只有 WHERE 条件

```
输入:  author == 'Tom' and year::INTEGER >= 2024

翻译:  SELECT path, name, mtime, size, tags FROM notes
       WHERE json_extract_string(properties, '$."author"') == 'Tom'
         AND json_extract_string(properties, '$."year"')::INTEGER >= 2024
```

### 表达式模式 — WHERE 条件 + 后续子句

```
输入:  list_contains(tags, 'todo') ORDER BY mtime DESC LIMIT 10

翻译:  SELECT path, name, mtime, size, tags FROM notes
       WHERE list_contains(tags, 'todo')
       ORDER BY mtime DESC
       LIMIT 10
```

### 表达式模式 — 只有后续子句

```
输入:  ORDER BY mtime DESC LIMIT 10

翻译:  SELECT path, name, mtime, size, tags FROM notes
       ORDER BY mtime DESC
       LIMIT 10
```

### SQL 模式

```
输入:  SELECT path, author, mtime FROM notes WHERE list_contains(categories, 'work') ORDER BY mtime DESC LIMIT 10

翻译:  SELECT path,
              json_extract_string(properties, '$."author"'),
              mtime
       FROM notes
       WHERE list_contains((properties->'$."categories"')::VARCHAR[], 'work')
       ORDER BY mtime DESC
       LIMIT 10
```

### IS NOT NULL

```
输入:  author IS NOT NULL

翻译:  SELECT path, name, mtime, size, tags FROM notes
       WHERE json_extract_string(properties, '$."author"') IS NOT NULL
```

### translate 子命令

```bash
$ markbase query translate "author == 'Tom' ORDER BY mtime DESC"

SELECT path, name, mtime, size, tags FROM notes
WHERE json_extract_string(properties, '$."author"') == 'Tom'
ORDER BY mtime DESC
```

### 安全拦截

```
输入:  DELETE FROM notes WHERE name == 'old-note'
输出:  Error: query command only supports SELECT statements, DELETE is not allowed
```

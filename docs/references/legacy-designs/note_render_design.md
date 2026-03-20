# `note render <n>` 命令设计说明

**状态：** 草案  
**目标系统：** markbase CLI  
**关联规范：** `docs/references/legacy-designs/template_schema.md`（MTS v1.11）、Obsidian Bases syntax

---

## 概述

`note render <n>` 命令将指定笔记完整渲染后输出到 stdout，供 agent 读取。渲染过程中，正文内容原样保留，其中的 `![[*.base]]` 嵌入会被替换为对应 base 的查询结果，并以注释行标注来源。

设计目标是让 agent 通过一条命令获得一篇笔记的完整上下文，包括 Obsidian 中由 base 动态展现的关联商机、相关人员等一对多关系数据。笔记中没有 `.base` 嵌入时，命令依然正常输出正文内容。

命令**不修改任何文件**，是只读、幂等操作。

---

## 命令格式

```bash
markbase note render <n> [-o table] [--dry-run]
```

**参数：**

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `<n>` | 必填 | note 的文件名（不含扩展名），与 `file.name` 一致。同时作为 `this` 的解析来源 |
| `-o table` | 选填 | 输出为 Markdown 表格格式，供人眼查看。默认为 list 格式 |
| `--dry-run` | 选填 | 不执行查询，在每个 view 位置输出将要执行的 SQL，用于调试 filter 翻译结果 |

> **设计决策：** `note render` 的主要目的是为 agent 提供笔记的完整上下文，默认输出 list 格式（与 `query -o list` 一致），每条 base 记录以 `---` 分隔、每个字段单独一行，方便 agent 按字段名读取。`-o table` 作为辅助选项，供人工偶尔查看使用。view 的 `type` 字段是 Obsidian 的展现配置，与输出格式无关，渲染时忽略。`-o json` 不支持，传入时静默降级为 list 格式。

> **直接渲染 `.base` 文件：** 若 `<n>` 参数对应的是一个 `.base` 文件（即 `name` 字段含扩展名如 `opps.base`），命令将跳过正文扫描步骤，直接执行该 `.base` 文件中的所有 view，等效于该文件作为独立 base 被渲染。此时 `this` 上下文指向该 `.base` 文件本身（`this.name` = `opps.base`）。

**退出码：**

| 退出码 | 含义 |
| --- | --- |
| `0` | 渲染完成（含部分 WARN，无 ERROR） |
| `1` | 存在至少一个 ERROR，导致渲染中止 |

---

## .base 文件格式说明

`.base` 文件是合法的 YAML 文件，顶层字段如下：

```yaml
filters:          # 可选，全局 filter，对所有 view 生效
  and:            # 或 or: / not:，值为列表
    - <filter>    # 字符串条件，或嵌套的 and/or/not 对象

properties:       # 可选，列的显示配置
  <列名>:
    displayName: <显示名>

views:            # 必填，view 列表
  - type: table   # Obsidian 展现类型，渲染时忽略
    name: <名称>  # view 显示名，用作 ## 标题
    filters:      # 可选，view 级 filter，与全局 filter AND 合并
      and:
        - <filter>
    order:        # 可选，SELECT 哪些列（显示字段列表）
      - file.name
      - note.stage
      - status    # bare 名等价于 note.status
    sort:         # 可选，ORDER BY
      - property: file.name
        direction: ASC   # 或 DESC
    limit: 10     # 可选，LIMIT
```

**关键字段说明：**

- `order`：字符串列表，定义渲染时显示哪些列（映射为 SELECT 子句）。
- `sort`：对象列表，每项含 `property`（列名，翻译规则与 `order` 相同）和 `direction`（`ASC`/`DESC`），映射为 ORDER BY 子句。与 `order` 是**独立字段**，互不影响。
- `groupBy`：当前版本**忽略**，不报错。
- `summaries`：当前版本**忽略**，不报错。
- `formulas`：当前版本**忽略**，不报错。

---

## 执行流程

### Step 0：定位 note

通过 `name = <n>` 查询数据库，获取该 note 的完整记录，供后续正文渲染和 `this` 解析使用。

```sql
SELECT path, folder, name, ext, size,
       CAST(ctime AS TEXT), CAST(mtime AS TEXT),
       to_json(tags), to_json(links), properties
FROM notes
WHERE name = ?
```

> **调用方式：** 使用 `db.query(&sql, "", usize::MAX)` 接口，返回 `(Vec<String>, Vec<Vec<String>>)`，即列名列表和行数据。不要套 `executor.rs` 的默认 LIMIT 1000。

- note 不存在 → 向 stderr 输出 ERROR 并返回 `Err`（由 main.rs 转为 exit code 1）：
  ```
  ERROR: note '<n>' not found in index. Run `markbase index` first.
  ```
- 多结果 → 同上报 ERROR（name 全局唯一，理论上不应发生）。

将查询结果构建为 `ThisContext`（见 §Filter 翻译 §1），供后续 `this` 解析使用。

### Step 1：读取并流式处理正文

读取 `<base_dir>/<note_path>` 对应的 `.md` 文件，用 `gray_matter` 解析后取 `content` 部分（frontmatter 已剥离），逐行扫描。

识别 `![[*.base]]` 嵌入行的正则：

```rust
static BASE_EMBED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^!\[\[([^\]]+\.base)\]\]\s*$").unwrap());
```

对每一行：
- **普通行**：原样输出到 stdout。
- **`![[*.base]]` 嵌入行**：替换为该 base 的渲染结果（见 Step 2–5）。
- **其他 `![[*]]` 嵌入**（非 `.base`）：原样输出，不处理。

### Step 2：加载 base 文件

从正文解析出的 embed 文件名（如 `customer-opportunities.base`）中，**注意**：非 `.md` 文件在数据库中 `name` 字段存储的是**含扩展名的完整文件名**，因此查询时不要去掉扩展名：

```sql
SELECT path FROM notes WHERE name = 'customer-opportunities.base'
```

- 查询无结果 → WARN 输出到 stderr，原位输出占位注释到 stdout，继续处理后续行：
  ```
  WARN: base file 'customer-opportunities.base' not found in index, skipping.
  ```
  占位注释（写入 stdout）：
  ```
  <!-- [markbase] base 'customer-opportunities.base' not found -->
  ```
- 读取 `path` 对应的文件内容，用 `serde_yaml::from_str::<serde_json::Value>()` 解析 YAML。
- 解析失败 → WARN 到 stderr，原位输出占位注释：
  ```
  <!-- [markbase] failed to parse 'customer-opportunities.base': <reason> -->
  ```
- 提取顶层 `filters`（全局，可选）、`properties`（列配置，可选）、`views`（可选）字段。

### Step 3：解析 `this`

`this` 指向执行命令时传入的 `<n>`。Step 0 查询数据库时已取得 `<n>` 的完整行，构建为 `ThisContext` 直接使用，不需要再次查询。

### Step 4：逐 view 翻译 filter + 构造 SQL

对 `views` 列表（若字段不存在或为空数组，跳过，不输出任何内容）：

1. 调用 `merge_filters(global_filter, view_filter, this, base_name, warnings)` 得到 WHERE 字符串。
2. 调用 `translate_columns(view.order, base_properties, base_name, warnings)` 得到 `Vec<ColumnMeta>`。
3. 调用 `translate_sort(view.sort, base_name, warnings)` 得到 ORDER BY 字符串。
4. 构造完整 SQL：
   ```sql
   SELECT <columns>
   FROM notes
   WHERE <where_clause>
   ORDER BY <order_exprs>
   LIMIT <limit>
   ```
   - `<columns>`：来自 `translate_columns` 的 SQL 表达式列表；若 `order` 字段不存在或为空，默认选取 `name, path, mtime`。
   - `WHERE`：`where_clause` 为 None 时省略。
   - `ORDER BY`：`translate_sort` 返回为空时省略。
   - `LIMIT`：取 view 的 `limit` 字段，不存在时省略。
   - `groupBy`、`summaries`、`formulas`：忽略，不报错。

### Step 5：执行视图并渲染输出

在 `![[*.base]]` 原位先输出来源注释行（stdout），再紧跟各 view 的渲染结果：

```
<!-- start: [markbase] rendered from <base-name>.base -->

## <view-name>

<rendered-output>

<!-- end: [markbase] rendered from <base-name>.base -->
```

`<view-name>` 取 view 的 `name` 字段；若无 `name`，用 base 文件名（含扩展名）代替。

**dry-run 模式**（`--dry-run`）：

```
<!-- start: [markbase] dry-run from <base-name>.base -->

## <view-name>

```sql
SELECT ...
FROM notes
WHERE ...
ORDER BY ...
LIMIT ...
```

<!-- end: [markbase] dry-run from <base-name>.base -->
```

**正常模式**：执行 SQL（调用 `db.query(&sql, "", usize::MAX)`），将结果转为渲染格式。

---

## Filter 翻译规范

### 1. ThisContext 结构

```rust
pub struct ThisContext {
    pub name: String,       // note 的 name 列值（不含扩展名）
    pub folder: String,     // note 的 folder 列值（如 "company/"，根目录为空字符串）
    pub path: String,
    pub ext: String,
    pub size: i64,
    pub ctime: String,      // 从数据库取出的文本形式
    pub mtime: String,
    pub tags: Vec<String>,
    pub links: Vec<String>,
}
```

### 2. Filter 结构解析

`.base` 文件中 `filters` 字段**本身**就是一个 filter object（如 `{"and": [...]}`），直接传入 `translate_filter`，不需要额外解包：

```rust
// 正确用法：
let global_filter = base_yaml.get("filters"); // Value::Object {"and": [...]} 或 None
let view_filter = view_yaml.get("filters");
merge_filters(global_filter, view_filter, this, base_name, warnings)
```

filter object 的递归结构：

```
Value::Object { "and": Value::Array([item, item, ...]) }  → AND 逻辑
Value::Object { "or":  Value::Array([item, item, ...]) }  → OR 逻辑
Value::Object { "not": Value::Array([item]) }             → NOT 逻辑
Value::String("status != \"done\"")                       → 字符串条件，词法解析
```

同一个 `and`/`or` 数组里，各元素既可以是字符串，也可以是嵌套的 `and`/`or`/`not` 对象，需要在迭代时判断类型分别处理。

单元素 `and`/`or`（列表只有一项时），生成 `(A)` 形式，不需要特殊处理。

**全局 filter + view filter 合并**：将两者都翻译为字符串后，用 AND 拼接：
```sql
(<global_where>) AND (<view_where>)
```
任一为 None 时仅取另一个。

### 3. 字符串条件的词法解析规则

单个字符串 filter 按以下优先级逐一尝试匹配，匹配成功即翻译，否则追加 WARN 并跳过：

#### 3.1 `link()` 函数识别（最高优先级，先于其他规则）

字符串中凡出现 `link(...)` 调用，先替换为对应的字面量：

| 表达式 | 替换为 |
| --- | --- |
| `link(this)` | wikilink 字符串 `[[<this.name>]]`，用于字符串比较 |
| `link("note-name")` | wikilink 字符串 `[[note-name]]`，用于字符串比较 |

替换后，后续规则按替换后的形式翻译。

> **实现说明：** markbase 在 `properties` JSON 中存储的 link 字段值是原始 YAML 字符串（如 `"[[绿米]]"`）。因此 `link(this)` 翻译为字面量 `'[[<n>]]'`，`link("activity_log")` 翻译为字面量 `'[[activity_log]]'`，用于等值比较或 `list_contains`。

#### 3.2 文件函数

| Base filter 字符串 | DuckDB 翻译 | 说明 |
| --- | --- | --- |
| `file.hasLink(this.file)` | `list_contains(links, '<n>')` | `this.file` 是对当前 note File 对象的引用，翻译时取 `this.name` |
| `file.hasTag("t1")` | `(list_contains(tags, 't1') OR array_any(tags, x -> x LIKE 't1/%'))` | 含嵌套 tag 匹配 |
| `file.hasTag("t1", "t2")` | `((list_contains(tags,'t1') OR ...) OR (list_contains(tags,'t2') OR ...))` | 多参数取任意匹配 |
| `file.inFolder("x")` | `(folder = 'x/' OR folder LIKE 'x/%')` | 入参无论是否带尾部斜杠，统一规范化为加 `/` 后比较 |
| `file.ext == "md"` | `ext = 'md'` | |
| `file.name == "x"` | `name = 'x'` | |
| `file.mtime > <date_expr>` | `mtime > <translated_date_expr>` | 见 §3.4 日期翻译 |
| `file.ctime > <date_expr>` | `ctime > <translated_date_expr>` | 同上 |

> **注意：** `file.hasLink()` 仅支持 `this.file` 参数。其他参数形式（如 `file.hasLink("OtherNote")`）追加 WARN 并跳过该条件。

#### 3.3 比较运算符（`note.*`、bare、`file.*` 属性）

属性名解析规则（与 `translator.rs` 保持一致）：
- `file.<prop>` → 直接列名（`file.name` → `name`，`file.mtime` → `mtime` 等）
- `note.<field>` → `json_extract_string(properties, '$."<field>"')`
- bare `<field>`（无任何前缀，包括含点号的 `meta.author`）→ 同 `note.<field>`，点号转为嵌套路径：`meta.author` → `json_extract_string(properties, '$."meta"."author"')`

| Base filter 字符串 | DuckDB 翻译 | 说明 |
| --- | --- | --- |
| `note.<f> == "v"` 或 `<f> == "v"` | `json_extract_string(properties,'$."<f>"') = 'v'` | |
| `note.<f> == link(this)` 或 `<f> == link(this)` | `json_extract_string(properties,'$."<f>"') = '[[<n>]]'` | link(this) 先替换（见 §3.1） |
| `note.<f> == link("x")` 或 `<f> == link("x")` | `json_extract_string(properties,'$."<f>"') = '[[x]]'` | 同上 |
| `note.<f> != "v"` | `json_extract_string(properties,'$."<f>"') != 'v'` | |
| `note.<f> > <num>` | `json_extract_string(properties,'$."<f>"')::DOUBLE > <num>` | 数值比较自动 cast |
| `note.<f> < <num>` | 同上 `<` | |
| `note.<f> >= <num>` | 同上 `>=` | |
| `note.<f> <= <num>` | 同上 `<=` | |
| `file.<prop> == "v"` | `<col> = 'v'` | file.* 直接列名 |

**字符串值转义**：比较值中若含单引号（如 `O'Brien`），替换为 `''`（两个单引号）。

#### 3.4 方法调用（`.isEmpty()`、`.contains()`）

| Base filter 字符串 | DuckDB 翻译 |
| --- | --- |
| `note.<f>.isEmpty()` 或 `<f>.isEmpty()` | `(json_extract_string(properties,'$."<f>"') IS NULL OR json_extract_string(properties,'$."<f>"') = '')` |
| `file.tags.isEmpty()` | `(tags IS NULL OR len(tags) = 0)` |
| `note.<f>.contains("x")` 或 `<f>.contains("x")` | 优先翻译为 list：`list_contains((properties->'$."<f>"')::VARCHAR[], 'x')`；若字段类型为 string，翻译为：`json_extract_string(properties,'$."<f>"') LIKE '%x%'`（MVP 阶段统一使用 list 翻译） |
| `note.<f>.contains(link("x"))` 或 `<f>.contains(link("x"))` | `list_contains((properties->'$."<f>"')::VARCHAR[], '[[x]]')` |
| `note.<f>.contains(link(this))` 或 `<f>.contains(link(this))` | `list_contains((properties->'$."<f>"')::VARCHAR[], '[[<n>]]')` |

#### 3.5 日期算术翻译

日期算术字符串格式：`"<数字><单位>"` 或 `"<数字> <单位>"`（数字与单位之间可有空格）。

解析方式：用正则 `^(\d+)\s*([a-zA-Z]+)$` 提取数字和单位词，按下表映射：

| Base 单位词 | DuckDB INTERVAL 单位 |
| --- | --- |
| `y` / `year` / `years` | `YEAR` |
| `M` / `month` / `months` | `MONTH` |
| `w` / `week` / `weeks` | `WEEK` |
| `d` / `day` / `days` | `DAY` |
| `h` / `hour` / `hours` | `HOUR` |
| `m` / `minute` / `minutes` | `MINUTE` |
| `s` / `second` / `seconds` | `SECOND` |

生成格式：`INTERVAL '<数字> <UNIT>'`

示例：
- `"30d"` → `INTERVAL '30 DAY'`
- `"1 year"` → `INTERVAL '1 YEAR'`
- `"2 weeks"` → `INTERVAL '2 WEEK'`

完整日期表达式翻译：

| Base 表达式 | DuckDB 翻译 |
| --- | --- |
| `now()` | `NOW()` |
| `today()` | `CURRENT_DATE` |
| `now() - "30d"` | `NOW() - INTERVAL '30 DAY'` |
| `now() - "1 year"` | `NOW() - INTERVAL '1 YEAR'` |
| `file.ctime > now() - "1 year"` | `ctime > NOW() - INTERVAL '1 YEAR'` |

#### 3.6 不支持的情况（WARN 并跳过该条件）

以下情况追加 WARN 到 stderr，跳过该条件，继续处理其他条件：

- `this.note.<field>` 等 frontmatter 属性（`this.note.*` 形式）
- `file.hasLink()` 参数非 `this.file`（如 `file.hasLink("OtherNote")`）
- `formula.*` 属性引用
- `string.containsAll()`、`string.containsAny()`、`string.startsWith()`、`string.endsWith()` 等未支持方法
- `file.backlinks.*` 相关 filter（性能敏感，MVP 不支持）
- 字符串条件无法按上述任何规则解析

WARN 格式：
```
WARN: unsupported filter '<condition>' in '<base-name>.base', condition ignored.
```

### 4. 列名翻译规则（`translate_columns`）

`translate_columns` 接收 view 的 `order` 字段（`Vec<String>`）和 base 顶层的 `properties` 对象，返回 `Vec<ColumnMeta>`。

#### 4.1 列名解析

| order 中的列名 | SQL 表达式 | 默认显示名 | is_name_col | is_list_col |
| --- | --- | --- | --- | --- |
| `file.name` | `name` | `name` | true | false |
| `file.path` | `path` | `path` | false | false |
| `file.mtime` | `mtime` | `mtime` | false | false |
| `file.ctime` | `ctime` | `ctime` | false | false |
| `file.size` | `size` | `size` | false | false |
| `file.ext` | `ext` | `ext` | false | false |
| `file.folder` | `folder` | `folder` | false | false |
| `file.tags` | `tags` | `tags` | false | **true** |
| `file.links` | `links` | `links` | false | **true** |
| `file.embeds` | `embeds` | `embeds` | false | **true** |
| `note.<f>` | `json_extract_string(properties,'$."<f>"')` | `<f>` | false | 见 §4.2 |
| bare `<f>`（无前缀） | 同 `note.<f>` | `<f>` | false | 见 §4.2 |
| `formula.*` | **跳过 + WARN** | — | — | — |

> **重要**：bare 列名（如 `name`、`tags`、`status`）无论是否与数据库列名相同，统一按 `note.<f>` 规则翻译为 `json_extract_string`，**不**翻译为直接列名。若需引用文件属性，必须加 `file.` 前缀。

若 `order` 字段不存在或为空数组，默认使用 `[file.name, file.path, file.mtime]`。

#### 4.2 is_list_col 判断

- `file.tags`、`file.links`、`file.embeds` 固定为 `true`。
- `note.<f>` / bare `<f>`：若 base 的 `properties` 块中该列名有定义（键名与 order 中的列名相同，如 `status` 或 `file.ext`），且定义中无特殊 `type` 标注，默认 `false`。MVP 阶段对 note 属性统一使用 `false`，字符串输出。

#### 4.3 displayName 查找

`properties` 块的键名与 `order` 中的列名**完全相同**（包括前缀，如 `file.ext` 对应键 `file.ext`，`status` 对应键 `status`）。查找时以 order 列名为 key：

```rust
// base_yaml["properties"]["file.name"]["displayName"]
// base_yaml["properties"]["status"]["displayName"]
```

找到 `displayName` 时，用其值覆盖默认显示名。

### 5. 排序翻译规则（`translate_sort`）

`translate_sort` 接收 view 的 `sort` 字段（YAML 中是对象列表），生成 ORDER BY 子句：

```yaml
sort:
  - property: file.name
    direction: ASC
  - property: note.stage
    direction: DESC
```

- `property` 的列名翻译规则与 `translate_columns` 中 `order` 列名完全相同（`file.*` → 直接列名，`note.*`/bare → `json_extract_string`）。
- `direction` 只接受 `ASC` 或 `DESC`（大小写不敏感），其他值 WARN 并默认使用 `ASC`。
- `sort` 字段不存在或为空数组时，返回空字符串，省略 ORDER BY 子句。

---

## 输出格式

### 整体结构

正文内容原样流式输出。每个 `![[*.base]]` 嵌入行被替换为来源注释行 + 各 view 渲染结果。

### name 字段的链接化

所有结果中，`file.name` 列（`is_name_col == true`）的值统一以 `[[name]]` 形式输出。其他字段不做此处理。空值不加括号。

### list 类型字段的输出

`is_list_col == true` 的字段（`file.tags`、`file.links`、`file.embeds`）：
- **list 格式**：字段名后换行，每个元素缩进两格输出 `  - value`
- **table 格式**：所有元素用 `, ` 拼接为单个单元格字符串

### 默认格式：list（key-value 块）

每条记录以 `---` 开头，每个字段单独一行，格式为 `字段名: 值`。字段显示名优先使用 base `properties` 中定义的 `displayName`，若无则使用原始列名。整个渲染结果以 YAML 代码块包裹，便于 agent 解析。

````
<!-- start: [markbase] rendered from customer-contacts.base -->

## 相关人员

```yaml
---
name: [[张三]]
title: 技术总监
role: Champion
aliases:
  - David Zhang
  - 张总
---
name: [[李四]]
title: 采购总监
role: Economic
aliases:
  - 李总
---
```

<!-- end: [markbase] rendered from customer-contacts.base -->
````

空结果集输出 `(no results)`。

### `-o table`：Markdown 表格

字段显示名规则同上。list 类型字段用 `, ` 拼接。

```
<!-- start: [markbase] rendered from customer-contacts.base -->

## 相关人员

| name     | title    | role     | aliases              |
|----------|----------|----------|----------------------|
| [[张三]] | 技术总监 | Champion | David Zhang, 张总    |
| [[李四]] | 采购总监 | Economic | 李总                 |

<!-- end: [markbase] rendered from customer-contacts.base -->
```

空结果集：输出表头 + `| (no results) |` 行。

### `--dry-run`：SQL 调试输出

正文普通行原样输出，每个 `![[*.base]]` 嵌入位置替换为该 base 各 view 将要执行的 SQL，以注释块包裹：

````
<!-- start: [markbase] dry-run from customer-opportunities.base -->

## 相关商机

```sql
SELECT name, json_extract_string(properties, '$."stage"'), mtime
FROM notes
WHERE list_contains(links, '绿米')
  AND json_extract_string(properties, '$."type"') = 'opportunity'
ORDER BY mtime ASC
LIMIT 10
```

<!-- end: [markbase] dry-run from customer-opportunities.base -->
````

`--dry-run` 与 `-o` 可同时指定，但 `-o` 在 dry-run 模式下无效（不影响 SQL 输出格式）。

---

## 列名翻译速查表

view `order`/`sort.property` 中的列名按以下规则翻译：

| 列名 | SQL 表达式 | 默认显示名 |
| --- | --- | --- |
| `file.name` | `name` | `name` |
| `file.path` | `path` | `path` |
| `file.mtime` | `mtime` | `mtime` |
| `file.ctime` | `ctime` | `ctime` |
| `file.size` | `size` | `size` |
| `file.ext` | `ext` | `ext` |
| `file.folder` | `folder` | `folder` |
| `file.tags` | `tags` | `tags` |
| `file.links` | `links` | `links` |
| `file.embeds` | `embeds` | `embeds` |
| `note.<field>` | `json_extract_string(properties, '$."<field>"')` | `<field>` |
| `<field>`（无前缀） | 同 `note.<field>` | `<field>` |
| `formula.*` | 不支持，跳过该列 + WARN | — |

`properties` 中若定义了 `displayName`，则用 `displayName` 覆盖默认显示名。

---

## MVP 范围边界

**明确支持：**
- 正文内容原样输出，`.base` 嵌入原位替换为渲染结果
- 无 `.base` 嵌入的笔记正常输出正文，不报错
- 所有 view `type`（渲染时忽略）
- 全局 `filters` + view 级 `filters` 的 AND 合并
- `link(this)` 和 `link("name")` 函数翻译为 wikilink 字符串字面量
- `file.hasLink(this.file)` — 实现一对多关系展现的核心 filter
- `file.hasTag`（含多参数、含嵌套 tag 匹配）、`file.inFolder`（含子目录，斜杠规范化）、`file.ctime`/`file.mtime` 的日期比较
- `note.<field>` 和 bare `<field>` 的基本比较（含 `link(this)` 和 `link("name")` 右值）
- `<field>.contains(link(this))`、`<field>.contains(link("name"))`、`<field>.contains("str")` 翻译为 `list_contains`
- `isEmpty()`、`now()` / `today()` + 日期算术（支持 `"30d"`、`"1 year"` 两种格式）
- `and` / `or` / `not` 嵌套逻辑，同一数组中字符串与对象混合
- view 的 `order`（SELECT 列定义）、`sort`（ORDER BY，独立字段）、`limit`
- bare 列名在 `order` 中翻译为 `note.*`（不按列名直接映射）
- `properties.displayName` 列名映射（键名与 order 列名完全匹配）
- `file.name` 列值统一以 `[[name]]` 链接形式输出
- `file.tags`、`file.links`、`file.embeds` 识别为 list 类型字段
- 默认 list + `-o table` + `--dry-run` 三种输出模式

**明确不支持（遇到时 WARN 到 stderr，不中断）：**
- `formulas`、`summaries`、`groupBy` — 忽略
- `formula.*` 列 — 跳过该列
- `this.note.<field>` 等 frontmatter 属性 — 条件忽略
- `file.hasLink()` 非 `this.file` 参数 — 条件忽略
- `file.backlinks.*` 相关 filter — 条件忽略
- `string.containsAll()`、`containsAny()`、`startsWith()`、`endsWith()` — 条件忽略
- 字符串 filter 解析失败 — 条件忽略
- `-o json` — 静默降级为 list

---

## 完整输出示例

假设 `绿米.md` 正文如下，其中 `customer-opportunities.base` 内容：

```yaml
views:
  - type: table
    name: 相关商机
    filters:
      and:
        - related_customer == link(this)
        - type == "opportunity"
    order:
      - file.name
      - stage
      - file.mtime
    sort:
      - property: file.mtime
        direction: DESC
    limit: 10
```

**`markbase note render 绿米`**（默认 list 格式）：

```
## 1. 公司简介

绿米联合创新科技，专注智能家居领域。

## 2. 相关商机

<!-- start: [markbase] rendered from customer-opportunities.base -->

## 相关商机

```yaml
---
name: [[绿米-商机2026]]
stage: Proposal
mtime: 2026-02-10
---
name: [[绿米-POC项目]]
stage: Negotiation
mtime: 2026-01-20
---
```

<!-- end: [markbase] rendered from customer-opportunities.base -->
```

**`markbase note render 绿米 --dry-run`**：

````
## 2. 相关商机

<!-- start: [markbase] dry-run from customer-opportunities.base -->

## 相关商机

```sql
SELECT name, json_extract_string(properties, '$."stage"'), mtime
FROM notes
WHERE json_extract_string(properties, '$."related_customer"') = '[[绿米]]'
  AND json_extract_string(properties, '$."type"') = 'opportunity'
ORDER BY mtime DESC
LIMIT 10
```

<!-- end: [markbase] dry-run from customer-opportunities.base -->
````

---

## 模块职责

| 模块 | 职责 |
| --- | --- |
| `src/renderer/mod.rs`（新增） | 核心渲染逻辑：Step 0–5，流式处理正文、识别并展开 base 嵌入 |
| `src/renderer/filter.rs`（新增） | Filter AST 解析与 DuckDB WHERE 子句翻译；列名翻译；sort 翻译；ThisContext 定义 |
| `src/renderer/output.rs`（新增） | list（key-value 块）和 table（Markdown 表格）格式化输出；ColumnMeta 定义 |
| `src/main.rs` | 注册 `NoteCommands::Render { name, format, dry_run }` 子命令，调用 renderer |
| `src/db.rs` | 复用现有 `db.query()` 接口 |

---

## 与现有命令的关系

| 命令 | 关系 |
| --- | --- |
| `markbase index` | `render` 依赖已建立的索引，建议在 render 前先 index |
| `markbase query` | filter 翻译逻辑与 query 的翻译层高度相似，遵循相同的字段名→SQL 规则；list 输出格式与 `query -o list` 完全一致 |
| `markbase note verify` | 独立命令，无直接依赖；可在同一 agent workflow 中组合使用 |

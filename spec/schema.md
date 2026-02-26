# MKS (Markdown Knowledge Schema) v1.10

**Status:** Stable / Production Ready  
**Date:** 2026-02-26  
**Target System:** mdb CLI, OpenClaw Agent  

---

## 1. 核心理念与流程 (Core Philosophy & Workflow)

MKS v1.10 是连接非结构化对话流与结构化知识库的协议。它不仅仅定义数据格式，更定义了知识的全生命周期管理流程：

1. **流式采集 (Stream Collection):**
   依靠 Frontmatter 中的 `description`（语义路由）和 `properties`（Prompt），指导 Agent 从杂乱的对话流中精准提取信息。

2. **实体对齐 (Entity Alignment):**
   依靠 `format: link` 和 `target` 约束，强制 Agent 将文本锚定到系统内的唯一实体文件（如 `[[绿米]]`），消除歧义。对于无法立即确认的实体，使用悬空引用语法 `[?[...]]` 占位。

3. **结构化沉淀 (Structured Sedimentation):**
   依靠模板正文中的指令，指导 Agent 在后续活动中，如何将碎片化信息回填到实体档案中，实现知识的动态生长。

---

## Part I: 骨架定义 (Frontmatter Schema)

位于 Frontmatter 的 `_schema` 属性下。**`_schema` 属性仅存在于模板文件中，实例文件不含此属性。**

### 1.1 根对象 (Root Object)

`_schema` 的定义与 **OpenAPI v3.1 / JSON Schema** 的标准对齐。

```yaml
_schema:
  # [1. 采集指引]
  # 必填。Agent 读取此描述，判断当前信息流是否匹配该模版。
  description: string

  # [2. 字段校验]
  # 默认为 false。
  # strict: true  — Agent 发现未定义字段时报错，拒绝写入。
  # strict: false — 未定义字段采取「只读透传（read-only passthrough）」策略：
  #                 读取内容纳入上下文，但不主动写入或修改。
  strict: boolean

  # [3. 数据完整性]
  # 必填字段的 Key 列表。创建文档时，若上下文无法自动推断，Agent 须向用户提问获取。
  # 提问时机：先完成实体对齐；仍无法确定时，再向用户提问。
  required: [string]

  # [4. 文件命名]
  # 可选。用自然语言描述文件名的推断规则，指导 Agent 在执行 mdb new 前确定文件名。
  # 若缺省，Agent 依据上下文自行判断。
  filename:
    description: string

  # [5. 存放路径]
  # 可选。固定目录路径（相对于 Vault 根目录），末尾须含 /。
  # 路径应固定，不支持动态分支。
  # 此字段由 mdb new 内部读取，Agent 无需关心路径拼接，只需提供文件名。
  location: string

  # [6. 结构定义]
  properties:
    <field_name>: <SchemaObject>
```

### 1.2 实例文件必填 Frontmatter

所有由模板创建的实例文件，Frontmatter 中必须包含以下两个字段。这两个字段直接定义在模板 Frontmatter 的外层（`_schema` 之外），`mdb new` 创建骨架时原样保留：

| 字段           | 说明                                                                                          | 示例                            |
| -------------- | --------------------------------------------------------------------------------------------- | ------------------------------- |
| **`type`**     | 实体类型。用于跨模板的实体对齐查询（`note.type == 'company'`）。                             | `type: company`                 |
| **`template`** | 所用模板的文件名（不含路径及md后缀，模板统一存放于 `templates/` 目录）。用于沉淀阶段精确反查模板指令。 | `template: company_customer` |

> **为何同时需要 `type` 和 `template`：** `type` 用于跨模板的实体查询（如「找所有 person 类型的实体」），`template` 用于精确反查沉淀指令（如同为 `person` 类型，客户人员模板和私人朋友模板的正文指令可能完全不同）。

### 1.3 属性定义 (Schema Object)

类型系统与 [Obsidian Properties](https://help.obsidian.md/properties) 对齐，并在 `format` 上做增强。

#### 类型定义（`type`）

| MKS `type`   | Obsidian UI 对应 | 说明                                     |
| ------------ | ---------------- | ---------------------------------------- |
| `text`       | Text             | 普通文本                                 |
| `number`     | Number           | 数值                                     |
| `boolean`    | Checkbox         | 布尔值，使用 `boolean` 更直观            |
| `date`       | Date             | 日期，格式 `YYYY-MM-DD`                  |
| `datetime`   | Date & time      | 日期时间，格式 `YYYY-MM-DDTHH:MM`        |
| `list`       | List             | 数组，含 aliases、tags 等列表类字段      |

#### 格式增强（`format`）

`format` 字段仅有一个合法值，专用于双链约束：

| `format` 值 | 说明                                                                                              |
| ----------- | ------------------------------------------------------------------------------------------------- |
| `link`      | 该字段值必须为 Obsidian 双链格式。配合 `target` 指定实体类型约束。仅可用于 `type: text` 或 `type: list` 的字段。 |

#### 完整 Schema Object 属性表

| 属性 Key          | 类型   | 描述                                                |
| ----------------- | ------ | --------------------------------------------------- |
| **`type`**        | string | 见上方类型定义表                                    |
| **`format`**      | string | 目前仅支持 `link`                                   |
| **`target`**      | string | 实体类型约束，仅当 `format: link` 时有效            |
| **`enum`**        | array  | 预设值列表，用于下拉约束和幻觉抑制                  |
| **`description`** | string | 字段填写的 Prompt 指引，指导 Agent 如何提取和填写   |
| **`default`**     | any    | 字段默认值，创建实例时若上下文无信息则使用此值      |

---

## Part II: 血肉定义 (Body Directives)

正文指令**只存在于模板文件**中，不随实例文件复制。Agent 在沉淀阶段通过实例文件的 `template` 字段精确反查对应模板，获取指令后操作实例文件正文。

### 2.1 指令语法

```markdown
## 章节标题
<!-- [Directive]: Policy
     详细的自然语言描述，说明 Agent 如何填写或更新此章节。
-->
```

### 2.2 核心指令集

| 指令 Key       | 作用阶段     | 描述                                                                |
| -------------- | ------------ | ------------------------------------------------------------------- |
| **`[Fill]`**   | **初始采集** | 指导 Agent 首次创建文档时，如何生成该段落内容。                     |
| **`[Update]`** | **后续沉淀** | 定义当新信息出现时，如何修改此段落。这是「结构化沉淀」的核心机制。 |

> **默认行为：** 模板中无任何指令的章节，Agent 不得写入，留空供人工填写。

### 2.3 更新策略 (`[Update]` Policies)

| 策略             | 行为                                                                                                                                                                                         | 适用场景               |
| ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------- |
| **`Overwrite`**  | 发现更新的信息时，完全重写该段落。                                                                                                                                                           | 简介、最新状态         |
| **`Append`**     | 在段落末尾追加新条目，保留完整历史。每条记录须携带时间戳。                                                                                                                                   | 活动日志、大事记       |
| **`Accumulate`** | 每次发现新信息均追加新条目并携带时间戳，保留全部历史。**不做覆盖或去重**，即使新旧信息指向同一实体的不同状态（如 `GitLab CE → GitLab Duo`），也各自成条，确保演进轨迹可追溯。              | 技术栈画像、关键人清单 |

> **幂等性保证：** 对于 `Append` 和 `Accumulate` 策略，Agent 在追加前须检查段落内是否已存在来源相同（相同源文件路径）的条目，若存在则跳过，防止重复触发产生重复记录。

### 2.4 悬空引用 (Dangling Reference)

当 Agent 在信息流中识别到一个应为 `format: link` 的实体，但**无法立即确认其对应的实体文件**时，使用悬空引用语法占位：

```
[?[David Chen]]
```

区别于已确认的 `[[David Chen]]` 双链，悬空引用表示「已识别但待对齐的实体」。

**解除悬空引用的流程：**
1. 人工（或 Agent）执行实体对齐，确认目标实体文件。
2. 将文件中的 `[?[David Chen]]` 替换为 `[[David Chen]]`。
3. 若需要，同步将原始文本添加为目标实体的 `aliases`，并重新执行 `mdb index`。

---

## Part III: 实体对齐算法 (Entity Alignment Algorithm)

当 Agent 处理 `format: link` 字段，或在沉淀阶段识别到需要链接的实体名时，执行以下流程。

### 步骤 1：搜索

对目标类型分两步查询，优先精确匹配，再尝试 aliases：

```bash
# 1a. 按文件名精确匹配（name 字段有索引，优先）
mdb query "name == '绿米' and note.type == 'company'" -o json

# 1b. 按 aliases 匹配（frontmatter list 字段，has() 支持）
mdb query "has(note.aliases, '绿米联合') and note.type == 'company'" -o json
```

### 步骤 2：结果处理

| 情况         | Agent 行为                                                                                                                                                                                                       |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **唯一命中** | 直接使用该实体，生成双链 `[[实体名]]`。                                                                                                                                                                          |
| **多结果**   | Agent 读取各候选文件的 `content` 字段，结合当前上下文自主推断最匹配项（如：某 `person` 文件记录了其所在公司，与当前商机吻合）。若仍无法确定，向用户列出候选项请求澄清。用户确认后，Agent 将原始文本添加为该实体的 `aliases`（见步骤 3）。 |
| **零结果**   | 写入悬空引用 `[?[实体名]]` 占位，不阻塞主流程。后续由人工或 Agent 批量处理。                                                                                                                                    |

> **与 `required` 字段的协作：** 若对齐失败的字段同时是 `required` 字段，Agent 须额外向用户提问，确保必填项不以悬空引用状态长期留存。

### 步骤 3：写入 alias（如适用）

用户指定别名关联到已有实体后，Agent 修改该实体文件的 Frontmatter，并执行 `mdb index` 使变更生效：

```yaml
# 修改前
aliases: ["绿米"]

# 修改后（用户用"绿米联合"指代同一实体）
aliases: ["绿米", "绿米联合"]
```

---

## Part IV: 文件创建流程 (File Creation Workflow)

实例文件**必须通过 `mdb new` 创建**，禁止直接复制模板文件，以确保 `_schema` 和正文指令不污染实例文件。

### 完整流程

```
Step 0  执行 mdb template list（每次会话开始时执行一次）
        ↓ 将返回的 name / description / path 载入上下文
        ↓ 作为后续路由判断和模板预读的索引

Step 1  Agent 预读模板文件（mdb new 之前）
        ↓ 记住：_schema.required、_schema.strict、_schema.properties
        ↓ 记住：_schema.filename.description（命名规则）
        ↓ 记住：各章节的 [Fill] / [Update] 指令内容

Step 2  确定文件名
        ↓ 依据 _schema.filename.description，从上下文推断文件名
        ↓ 若 _schema.filename 缺省，Agent 自行判断
        ↓ 无法推断文件名时，向用户提问

Step 3  mdb new <实例名> --template <模板文件名>
        ↓ mdb 从模板的 _schema.location 自动确定存放路径，Agent 无需干预
        ↓ mdb 过滤掉 _schema 属性和所有 <!-- [...] --> 指令注释
        ↓ 保留模板外层 Frontmatter（含 type、template 等字段）原样写入骨架文件
        ↓ 生成干净的骨架文件，命令返回新文件的完整路径

Step 4  Agent 填充 Frontmatter
        ↓ 从上下文推断各字段值
        ↓ 对 format: link 字段执行实体对齐（Part III）
        ↓ required 字段若无法推断，向用户提问
        ↓ 有 default 值的字段，上下文无信息时使用默认值

Step 5  Agent 填充正文
        ↓ 依据 Step 1 记住的 [Fill] 指令，填充对应章节
        ↓ 无 [Fill] 指令的章节保持空白，不写入
```

---

## Part V: 完整模版范例 (Reference Template)

> 引用文件：`@/templates/company_customer.md`

---

## Part VI: Agent 工作流算法 (The Algorithm)

### 阶段 0：模板感知 (Template Awareness)

**每次会话开始时执行一次**，为后续所有阶段建立路由基础：

```bash
mdb template list
```

输出示例：
```
name: person_work
_schema.description: 工作相关人员档案模板。适用于客户侧联系人、合作伙伴...
path: /vault/templates/person_work.md
---
name: company_customer
_schema.description: 标准客户档案模版。用于建立新客户的基本信息库...
path: /vault/templates/company_customer.md
---
```

Agent 将返回的 `name`、`description`、`path` 载入上下文，作为后续路由判断和模板预读的索引。

---

### 阶段 1：流式采集 (Collection)

1. **用户输入：** "记录一下，今天拜访了绿米..."
2. **路由：** 依据阶段 0 已加载的模板列表，匹配 `_schema.description`，选中目标模板（此例为 `meeting_log`）。
3. **关联实体预检（Company Prefetch）：**
   - 识别对话中涉及的 company 实体（此例为「绿米」）。
   - 执行实体对齐查询（Part III 算法）：
     - **命中** → 读取该 company 文件的完整内容载入上下文（包含已有的商机状态、关键人、技术栈等），辅助后续实体识别和信息填充。
     - **未命中** → 先触发 `company_customer` 模板的完整创建流程（阶段 0 → 阶段 1），创建完成后再继续当前文档的创建。
4. **预读模板：** 读取目标模板文件，记住 `_schema`（含 `filename`）和所有正文指令。
5. **确定文件名：** 依据 `_schema.filename.description` 从上下文推断文件名，无法推断时向用户提问。
6. **必填校验：** 对 `required` 字段，先尝试从上下文推断；推断不足时，先走实体对齐；仍无法确定时，向用户提问。
7. **创建文件：** 调用 `mdb new`，基于模板生成骨架文件。
8. **填充内容：** 依据预读的指令填充 Frontmatter 和 `[Fill]` 章节。

---

### 阶段 2：实体对齐 (Alignment)

1. 读取模板 `properties`，找到所有 `format: link` 字段。
2. 对每个字段，按 Part III 算法依次执行：文件名精确匹配 → aliases 匹配 → 结果处理。
3. 唯一命中写双链；多结果推断或询问；零结果写悬空引用。
4. 示例：生成会议纪要 `logs/2026-02-26_绿米拜访.md`，写入 `related_customer: [[绿米]]`。

---

### 阶段 3：结构化沉淀 (Sedimentation)

*此阶段由后台任务或 Agent 的「反思」步骤触发*

1. **触发：** 检测到新创建的会议纪要关联了 `[[绿米]]`。
2. **加载实例：** 读取 `company/绿米.md`，获取 `template: company_customer`。
3. **查模板：** 在阶段 0 的模板索引中定位 `company_customer` 的 `path`，读取模板加载全部正文指令。
4. **逐章扫描，按指令执行：**

   | 章节               | 指令                   | 操作                                                                                        |
   | ------------------ | ---------------------- | ------------------------------------------------------------------------------------------- |
   | `## 2. 组织架构`   | `[Update]: Accumulate` | 提取会议纪要中出现的人员，实体对齐后追加（不确定者写悬空引用）；幂等检查通过后写入         |
   | `## 3. 技术栈画像` | `[Update]: Accumulate` | 提取新技术信息，带时间戳追加；幂等检查通过后写入                                           |
   | `## 4. 关键活动`   | `[Update]: Append`     | 幂等检查通过后，追加：`- [[2026-02-26]] [Visit] 讨论私有化部署 → [[logs/2026-02-26_绿米拜访]]` |

5. **重新索引：** 若有 alias 写入，执行 `mdb index` 使变更生效。

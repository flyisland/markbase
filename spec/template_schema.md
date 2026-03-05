# MTS (Markdown Template Schema) v1.11

**Status:** Stable / Production Ready  
**Date:** 2026-03-04  
**Target System:** markbase CLI, OpenClaw Agent  

---

## Overview

MTS v1.11 defines a schema format for Markdown-based knowledge vaults. It specifies how templates encode field constraints, how body directives instruct agents to fill and update content, and how instance files are structured after creation.

The schema serves three roles:

1. **Semantic routing** — `_schema.description` lets an agent match incoming information to the correct template.
2. **Field constraints** — `_schema.properties` defines types, formats, enums, and prompts for each frontmatter field.
3. **Body directives** — `[!agent-fill]` and `[!agent-update]` callouts embedded in the template body are copied into instance files, instructing agents how to generate and update each section.

For agent operational procedures (capture, entity alignment, knowledge consolidation), see `SKILL.md`.

---

## Part I: Frontmatter Schema

The schema is defined under the `_schema` key in the frontmatter. **`_schema` exists only in template files and is stripped from all instance files.**

### 1.1 Root Object

`_schema` follows the **OpenAPI v3.1 / JSON Schema** conventions.

```yaml
_schema:
  # [1. Routing prompt]
  # Required. The Agent reads this to decide whether the current information
  # stream matches this template.
  description: string

  # [2. Field validation]
  # Default: false.
  # strict: true  — The Agent rejects writes for any field not defined in properties.
  # strict: false — Undefined fields follow a read-only passthrough policy:
  #                 their content is loaded into context but never written or modified.
  strict: boolean

  # [3. Data integrity]
  # List of required field keys. If the Agent cannot infer a value from context,
  # it must ask the user. Entity alignment should be attempted first;
  # only ask the user if alignment also fails.
  required: [string]

  # [4. File naming]
  # Optional. A natural-language rule describing how to derive the filename.
  # If omitted, the Agent uses its own judgment.
  filename:
    description: string

  # [5. Storage location]
  # Optional. A fixed directory path relative to the vault root, ending with /.
  # Paths must be static — dynamic branching is not supported.
  # markbase note new reads this internally; the Agent only needs to provide the filename.
  location: string

  # [6. Field definitions]
  properties:
    <field_name>: <SchemaObject>
```

### 1.2 Required Instance Frontmatter

Every instance file created from a template must include the following two fields. Both are defined in the outer frontmatter of the template (outside `_schema`) and are copied as-is by `markbase note new`:

| Field           | Description | Example |
| --------------- | ----------- | ------- |
| **`type`**      | Entity type. Used for cross-template entity alignment queries (e.g. `note.type == 'company'`). | `type: company` |
| **`templates`** | Internal link(s) to the template(s) used to create this file (`list` type, `format: link`). All templates live in the `templates/` directory. Each link element must be quoted. | `templates: ["[[company_customer]]"]` |

`type` enables cross-template entity queries. `templates` allows the Agent to look up the `_schema` definition when needed. The internal link format also lets Obsidian navigate directly to the template file.

### 1.3 Schema Object

The type system aligns with [Obsidian Properties](https://help.obsidian.md/properties), with an additional `format` extension.

#### Field Types (`type`)

| MTS `type` | Obsidian UI | Description |
| ---------- | ----------- | ----------- |
| `text`     | Text        | Plain text |
| `number`   | Number      | Numeric value |
| `boolean`  | Checkbox    | Boolean value |
| `date`     | Date        | Date in `YYYY-MM-DD` format |
| `datetime` | Date & time | Date and time in `YYYY-MM-DDTHH:MM` format |
| `list`     | List        | Array, including fields like `aliases`, `tags` |

#### Format Extension (`format`)

| `format` value | Description |
| -------------- | ----------- |
| `link` | The field value must be an Obsidian internal link. Use `target` to constrain the entity type. Valid only on `text` or `list` fields. |

#### Schema Object Properties

| Key | Type | Description |
| --- | ---- | ----------- |
| **`type`** | string | See field types table above. |
| **`format`** | string | Currently only `link` is supported. |
| **`target`** | string | Entity type constraint, valid only when `format: link`. The value corresponds to the `type` field of instance files (e.g. `company`, `person`). |
| **`enum`** | array | List of allowed values. Used to constrain input and reduce hallucination. |
| **`description`** | string | A prompt that tells the Agent how to extract and fill this field. |
| **`default`** | any | Default value to use when no information is available in context at creation time. |

---

## Part II: Body Directives

Directives are embedded in the template body as **Agent Callouts** and are copied as-is into instance files by `markbase note new`. The Agent reads and executes directives directly from the instance file — no template lookup is required during either collection or consolidation.

### 2.1 Directive Syntax

MTS uses two dedicated [Obsidian Callout](https://help.obsidian.md/callouts) types:

#### `[!agent-fill]` — Initial collection directive

```markdown
## Section Title

> [!agent-fill]-
> Natural-language description of how the Agent should generate this section when first creating the document.
```

#### `[!agent-update]` — Consolidation directive

```markdown
## Section Title

> [!agent-update]- Accumulate
> Natural-language description of how the Agent should update this section when new information arrives.
```

The callout title of `[!agent-update]` specifies the update policy. See §2.3 for valid values.

### 2.2 Directive Types

| Callout type | Phase | Description |
| ------------ | ----- | ----------- |
| **`agent-fill`** | Initial collection | Instructs the Agent how to generate this section when first creating the document. The callout is retained after execution. |
| **`agent-update`** | Consolidation | Defines how to update this section when new information arrives. The callout is retained after execution. |

> **Default behavior:** Sections with no Agent Callout must not be written by the Agent. They are left blank for manual editing.

### 2.3 Update Policies (`[!agent-update]` title)

| Policy | Behavior | Typical use |
| ------ | -------- | ----------- |
| **`Overwrite`** | Completely rewrites the section when updated information is found. | Summary, current status |
| **`Append`** | Appends new entries at the end of the section, preserving full history. Each entry must include a timestamp. | Activity logs, timelines |
| **`Accumulate`** | Appends a new timestamped entry every time new information is found, with no deduplication. Even when new and old information refer to the same entity in different states (e.g. `GitLab CE → GitLab Duo`), both are kept. | Tech stack, key contacts |

### 2.4 Callout Lifecycle

| Phase | Template file | Instance file |
| ----- | ------------- | ------------- |
| `markbase note new` creates instance | Callout is copied as-is | Callout is present, awaiting execution |
| Collection (`agent-fill`) | — | Agent fills section content. **Callout is retained.** |
| Consolidation (`agent-update`) | — | Agent updates section per policy. **Callout is retained.** |

Callouts are never removed after execution. This gives humans a permanent record of the intended behavior for each section.

### 2.5 Dangling References

When an entity should be a `format: link` value but cannot be confirmed, a dangling reference is written as a placeholder:

```
[?[David Chen]]
```

This is distinct from a confirmed internal link `[[David Chen]]`. To resolve:
1. Perform entity alignment and confirm the target file.
2. Replace `[?[David Chen]]` with `[[David Chen]]`.
3. If needed, add the original text as an `aliases` entry on the target entity.
4. Run `markbase index` to apply all changes.

---

## Part III: Reference Template

The following is a complete example of the `company_customer` template, showing how `_schema`, outer frontmatter, and Agent Callouts work together.

```markdown
---
_schema:
  description: 标准客户档案模版。用于建立新客户的基本信息库，记录组织架构、技术栈和关键活动。
  strict: false
  required: [industry, size]
  filename:
    description: 使用客户的常用简称作为文件名，如"绿米"而非"绿米联合创新科技有限公司"。
  location: company/
  properties:
    industry:
      type: text
      description: 客户所在行业，如"智能家居"、"金融科技"。
    size:
      type: text
      enum: [startup, smb, enterprise]
      description: 公司规模，从枚举值中选择最匹配的。
    website:
      type: text
      description: 官网 URL。
    related_contacts:
      type: list
      format: link
      target: person
      description: 该客户的已知联系人，每人一个双链。

type: company
templates: ["[[company_customer]]"]
industry: ""
size: ""
website: ""
related_contacts: []
tags: []
aliases: []
---

## 1. 公司简介

> [!agent-fill]-
> 用 2-3 句话概括公司的主营业务、市场定位和核心产品。信息来源优先使用对话上下文，不足时可结合 website 字段访问官网补充。

## 2. 组织架构

> [!agent-update]- Accumulate
> 每次发现新的联系人信息时追加一条记录，格式为：
> `- [[人员双链]] · 职位 · 备注（来源：[[源文件]]，日期）`
> 幂等检查：若该来源文件已存在记录则跳过。无法确认人员实体时写悬空引用 [?[姓名]]。

## 3. 技术栈画像

> [!agent-update]- Accumulate
> 每次发现新的技术信息时追加一条记录，格式为：
> `- 技术名称 · 用途说明（来源：[[源文件]]，日期）`
> 即使同一技术出现版本迭代（如 GitLab CE → GitLab Duo），也各自成条保留演进轨迹。
> 幂等检查：若该来源文件已存在记录则跳过。

## 4. 关键活动

> [!agent-update]- Append
> 每次有新的拜访、会议、演示等活动时，在末尾追加一条记录，格式为：
> `- [[日期]] [活动类型] 简要描述 → [[源文件]]`
> 活动类型枚举：Visit / Call / Demo / Proposal / Contract。
> 幂等检查：若该来源文件已存在记录则跳过。

## 5. 商机状态

> [!agent-update]- Overwrite
> 每次获得更新的商机进展时，完整重写本节内容，保留最新状态即可，无需保留历史。
> 格式参考：阶段（如 Qualification / Proposal / Negotiation）、预计金额、预计签约时间、下一步行动。
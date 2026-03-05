# `note verify <n>` 命令设计说明

**状态：** 草案  
**目标系统：** markbase CLI  
**关联规范：** `spec/template_schema.md`（MTS v1.11）

---

## 概述

`note verify <n>` 命令用于校验一个 note 是否符合其所引用的 MTS 模板约束。它会逐项检查目录结构、frontmatter 字段和 `_schema.properties` 属性定义，并以警告（WARN）或错误（ERROR）的形式输出问题清单，供人工或 Agent 修正。

命令**不修改任何文件**，只做只读校验，是幂等操作。

---

## 命令格式

```bash
markbase note verify <n>
```

**参数：**

| 参数 | 说明 |
| --- | --- |
| `name` | note 的文件名（不含扩展名），与 `file.name` 一致 |

**退出码：**

退出码反映校验完成后的最终结果，与中间步骤是否提前退出无关：

| 退出码 | 含义 |
| --- | --- |
| `0` | 校验通过，无任何问题，或仅有 WARN（无 ERROR） |
| `1` | 存在至少一个 ERROR |

> **说明：** ERROR 表示流程无法继续（如找不到 note 或 template 文件）或存在严重校验失败；WARN 表示校验发现的字段级问题，不中止流程。两者均会被计入最终 summary。仅有 WARN 时命令返回退出码 0，遵循 AGENTS.md §15.3 规范。

---

## 执行流程

### Step 0：定位 note

通过 `file.name == <n>` 查询数据库，找到对应的 note 记录。

- 查询结果为空，输出 **ERROR** 并退出：
  ```
  ERROR: note '<n>' not found in index. Run `markbase index` first.
  ```
- 返回多个结果（理论上不应发生，因 name 全局唯一），输出 **ERROR** 并退出。

---

### Step 1：检查 `templates` 字段

从 note 的 frontmatter 中读取 `templates` 字段。

- `templates` 字段不存在，或值不是数组，或数组为空，输出 **ERROR** 并退出：
  ```
  ERROR: note '<n>' has no 'templates' field. Cannot determine schema.
  ```
- `templates` 数组中存在无法解析为 wiki-link 格式（`[[...]]`）的元素，输出 **ERROR** 并退出：
  ```
  ERROR: 'templates' contains invalid link: '<value>'. Each element must be an Obsidian wiki-link, e.g. "[[template-name]]".
  ```

`templates` 字段格式为 `list` + `format: link`，例如：
```yaml
templates: ["[[company_customer]]"]
```

每个元素是一个指向 `templates/` 目录下模板文件的 wiki-link。命令将对列表中每个 template **依次独立执行** Step 2–5，所有 template 的问题汇总后统一输出。

---

### Step 2：加载 template 文件

从 `templates` 字段中解析 wiki-link，提取 template 的 `name`（去掉 `[[` `]]`），然后在 `templates/` 目录下查找对应的 `.md` 文件。

- template 文件不存在，输出 **ERROR** 并退出（整个命令终止，不继续处理其余 template）：
  ```
  ERROR: template file 'templates/<template-name>.md' not found.
  ```
- 读取 template 文件，解析其 frontmatter，提取 `_schema` 对象备用。
- 如果 template 文件没有 `_schema` 字段，视为无 schema 约束，跳过 Step 3–5，继续下一个 template。

**多 template 冲突检测：** 在所有 template 加载完成后，若两个 template 对同一字段定义了不同的 `type`，输出 **WARN**：
```
WARN: field '<field>' has conflicting type definitions across templates
      ('<template-a>': '<type-a>', '<template-b>': '<type-b>'). Using '<template-a>' definition.
```
冲突时以列表中靠前的 template 定义为准，继续执行校验。

---

### Step 3：校验目录结构（location 约束）

读取 template 的 `_schema.location` 值（如有）。

`_schema.location` 为相对于 vault 根目录的路径前缀，例如 `company/`。

校验规则：note 的实际 `file.folder` 必须与 `_schema.location` 匹配，即 note 的路径应满足：

```
<vault-root>/<location>/<n>.md
```

- `_schema.location` 存在，但 note 的 `file.folder` 不符合，输出 **WARN** 并继续：
  ```
  WARN: note '<n>' is located at '<actual-folder>/',
        but template '<template-name>' requires location '<location>'.
  ```

---

### Step 4：校验模板 frontmatter 非 `_schema` 字段

读取 template frontmatter 中除 `_schema` 以外的所有字段（即实例文件应继承的字段，包括 `type`、`templates`、`industry` 等）。

对每个模板字段执行以下检查：

#### 4.1 字段存在性

- note 缺少该字段，输出 **WARN**：
  ```
  WARN: missing field '<field>' (defined in template '<template-name>').
  ```

#### 4.2 非 list 字段的值一致性

字段类型不是 list 时（模板中该字段的值是一个标量），这类字段代表**实例的固定值**（如 `type: company`），note 的值必须与模板完全一致。

- 值不同时，输出 **WARN**：
  ```
  WARN: field '<field>' value mismatch.
        Expected: '<template-value>' (from template '<template-name>'), got: '<note-value>'.
  ```

> **注意：** 若模板中该字段的值为空字符串或 null，跳过此检查（空值视为无约束）。

#### 4.3 list 字段的包含性

字段类型为 list 时，检查 note 的数组是否**包含**模板数组中的所有元素。`templates` 字段本身也适用此规则——note 的 `templates` 数组只需包含（而非完全等于）模板中定义的值。

- note 数组缺少模板中的元素，输出 **WARN**：
  ```
  WARN: list field '<field>' is missing values required by template '<template-name>'.
        Missing: [<value1>, <value2>, ...]
  ```

---

### Step 5：校验 `_schema.properties` 字段

读取 `_schema.properties`，对每个定义的属性执行以下检查。

#### 5.1 required 字段存在性

读取 `_schema.required` 列表，对其中每个字段名，检查 note 的 frontmatter 是否包含该字段且值非空。

- 字段缺失或为空，输出 **WARN**：
  ```
  WARN: required field '<field>' is missing or empty (defined in _schema.required of '<template-name>').
  ```

#### 5.2 类型校验

对 `_schema.properties` 中每个在 note 里**实际存在**的字段，检查其值是否符合 schema 定义的 `type`。

类型判断规则：

| MTS `type` | 判断方式 |
| --- | --- |
| `text` | 值为字符串 |
| `number` | 值可解析为数字 |
| `boolean` | 值为 `true` 或 `false` |
| `date` | 值符合 `YYYY-MM-DD` 格式 |
| `datetime` | 值符合 `YYYY-MM-DDTHH:MM` 格式 |
| `list` | 值为数组 |

- 类型不符合，输出 **WARN**：
  ```
  WARN: field '<field>' type mismatch.
        Expected '<type>' (from template '<template-name>'), got '<actual-type>'.
  ```

#### 5.3 enum 校验

若 schema 定义了 `enum`，检查字段值是否在枚举列表内。对于 `list` 类型，检查每个元素是否都在枚举内。

- 值不在枚举内，输出 **WARN**：
  ```
  WARN: field '<field>' has invalid value '<value>'.
        Allowed values (from template '<template-name>'): [<enum1>, <enum2>, ...]
  ```

#### 5.4 link 格式与 target 校验

对 `format: link` 的字段（`text` 或 `list` 类型），检查每个值是否为合法的 wiki-link 格式，以及 link 指向的 note 的 `type` 字段是否匹配 schema 中定义的 `target`。

**字段值为空（空字符串或空数组）时跳过此步骤**，由 5.1 的 required 检查覆盖。

校验步骤：
1. 判断字段值（或数组中每个元素）是否符合 `[[note-name]]` 格式。悬空引用 `[?[...]]` 跳过 target 检查，仅记录一条 INFO。
2. 提取 link 的 target name，查询数据库确认该 note 是否存在。
3. 若 schema 定义了 `target`，检查目标 note 的 `note.type` 是否等于该值。

- 值不是合法 wiki-link 格式，输出 **WARN**：
  ```
  WARN: field '<field>' has invalid link format: '<value>'.
        Expected Obsidian wiki-link, e.g. [[note-name]].
  ```
- 目标 note 不存在于 vault，输出 **WARN**：
  ```
  WARN: field '<field>' links to '<target-name>' which is not found in the vault.
  ```
- 目标 note 的 `type` 不匹配，输出 **WARN**：
  ```
  WARN: field '<field>' links to '<target-name>' (type: '<actual-type>'),
        but template '<template-name>' requires target type '<expected-type>'.
  ```

---

## 输出格式

校验过程中问题按发现顺序实时输出。完成后输出 summary 行。

**全部通过：**
```
✓ note 'acme' passed all checks against: company_customer.
```

**仅有 WARN（无 ERROR）：**
```
Verifying note 'acme' against template(s): company_customer

  [WARN] note 'acme' is located at 'contacts/', but template 'company_customer' requires location 'company/'.
  [WARN] required field 'size' is missing or empty (defined in _schema.required of 'company_customer').
  [WARN] field 'size' has invalid value 'unknown'. Allowed values (from template 'company_customer'): [startup, smb, enterprise]
  [WARN] field 'related_contacts' links to 'david-chen' (type: 'meeting'), but template 'company_customer' requires target type 'person'.

Verification completed with issues: 0 error(s), 4 warning(s).
```

**存在 ERROR（可能同时有 WARN）：**
```
Verifying note 'acme' against template(s): company_customer, person_contact

  [WARN] field 'industry' has conflicting type definitions across templates ('company_customer': 'text', 'person_contact': 'list'). Using 'company_customer' definition.
  [WARN] missing field 'industry' (defined in template 'company_customer').
  [ERROR] template file 'templates/person_contact.md' not found.

Verification failed: 1 error(s), 2 warning(s).
```

**Summary 行措辞规则：**

| 情况 | Summary 行 |
| --- | --- |
| 无任何问题 | `✓ note '<n>' passed all checks against: <templates>.` |
| 仅有 WARN | `Verification completed with issues: 0 error(s), N warning(s).` |
| 有 ERROR | `Verification failed: N error(s), M warning(s).` |

---

## 模块职责建议

| 模块 | 职责 |
| --- | --- |
| `src/verifier.rs`（新增） | 实现核心校验逻辑，返回结构化的 `VerifyResult`（errors + warnings 列表） |
| `src/main.rs` | 新增 `NoteCommands::Verify { name }` 分支，调用 `verifier::verify_note()` 并格式化输出，根据结果设置退出码 |
| `src/db.rs` | 复用现有 query 接口查询 note 和 target note 信息 |

---

## 与现有命令的关系

| 命令 | 关系 |
| --- | --- |
| `markbase index` | `verify` 依赖已建立的索引；建议在 verify 前先 index |
| `markbase note new --template` | `new` 创建的 note 应能通过 `verify` 校验；两者约束来源相同（`_schema`） |
| `markbase template describe` | 可用于在 verify 前查看 template 的完整 schema 定义 |

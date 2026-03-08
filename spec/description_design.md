# `description` 字段设计说明

**状态：** 草案  
**日期：** 2026-03-08  
**目标系统：** markbase CLI

---

## 1. 背景

`markbase note resolve` 已经提供了基于 `file.name` 与 `aliases` 的实体对齐能力，但在 Agent 实际使用中，仅凭名称、别名和 `type` 仍然不足以稳定消歧，尤其是在以下场景中：

- `aliases` 命中只有 1 条记录，但该别名本身语义松散，不能直接视为高置信匹配。
- 同名或近义实体可能分布在不同模板、不同业务上下文中，仅靠文件名难以快速判断是否为目标 note。
- Agent 为了确认某个候选是否正确，往往需要额外执行 `markbase note render <name>`，增加了命令调用次数和心智负担。
- 对于普通笔记、临时笔记、日志类 Markdown note，即便没有强结构需求，Agent 仍然需要一个最小语义摘要来判断“这个 note 是什么”。

目前 vault 中并没有统一要求所有 Markdown note 都具备可供快速识别的摘要字段。这导致：

1. `resolve` 的返回结果缺乏足够上下文；
2. Agent 很难在“继续使用候选 note”与“继续消歧/追问用户”之间稳定决策；
3. 不同模板之间的数据完整性标准不一致。

---

## 2. 目标

为所有 Markdown note 引入统一的 frontmatter 字段 `description`，并将其作为 **最低成本的语义识别信息** 使用。

这里的 “Markdown note” 指：

- 所有 `.md` 笔记文件
- `templates/` 下的 Markdown 模板文件
- 不包含图片、PDF、音频等非 Markdown 资源

该特性的目标是：

1. **统一最小语义摘要**：所有 Markdown note 都必须具备一句话级别的 `description`。
2. **降低 Agent 对齐成本**：`resolve` 输出中可直接携带 `description`，减少额外 `render` 的需求。
3. **提升 alias 命中的可用性**：`alias` 命中不能直接视为安全，但可以先结合 `description` 与上下文做轻量判断。
4. **统一模板约束**：不再区分“实体类 note”与“普通 note”是否需要 `description`，一刀切要求，简化实现与使用规则。
5. **提升长期可维护性**：后续若扩展搜索、推荐、摘要展示、列表输出，均可复用同一字段。
6. **提升默认查询可读性**：`query` 在默认输出列中应包含 `description`，让 Agent 在表达式模式下无需额外指定列就能看到最小语义摘要。

---

## 3. 非目标

本设计**不**包含以下内容：

- 不引入 `description` 自动生成器；生成内容仍由模板、用户输入或 Agent 填写。
- 不修改数据库 schema；`description` 作为普通 frontmatter 字段继续存储在 `properties` JSON 中。
- 不改变 `file.name` 全局唯一原则。
- 不让 `description` 参与唯一性判断、重名判断或链接语法。
- 不在本次设计中规定 `description` 的最大长度、语言、风格模板，只要求它能够帮助快速识别 note。
- 不在本次设计中要求“读取模板时自动回写模板文件”；模板兜底仅发生在内存中的归一化过程。

---

## 4. 设计决策

### 4.1 字段定义

新增统一 frontmatter 字段：

```yaml
---
description: 一句话说明这个 note 是什么
---
```

约束：

- 字段名固定为 `description`
- 类型为 `text`
- 所有 Markdown note 必须具备该字段
- 语义目标为非空、可辨识的字符串

### 4.2 为什么一刀切要求所有 Markdown note

选择“所有 Markdown note 都必须有 `description`”而不是“仅实体类模板要求”的原因：

- 实现最简单，规则最统一
- Agent 不必判断某一类 note 是否“理论上应该有 description”
- `resolve` 可以稳定输出 `description`
- `verify` 规则可以写成全局约束，减少模板分支
- 普通 note 也可以使用简单值，例如：
  - `临时笔记`
  - `个人日记`
  - `待整理记录`

### 4.3 `resolve` 输出契约

引入 `description` 后，`note resolve` 返回的每个 match 都应包含稳定字段 `description`。

约束：

- 字段名固定为 `description`
- 类型为 `string | null`
- 若 note 缺少 `description`，输出 `null`
- 不因字段缺失而省略该 key

选择显式 `null` 而不是“省略字段”的原因：

- Agent 与脚本可以依赖稳定 JSON shape
- 消费方不需要额外判断字段是否存在
- 便于后续扩展排序、评分、列表展示等逻辑

### 4.4 `query` 默认输出列

`markbase query` 在空输入或表达式模式下，会使用一组默认 SELECT 列。引入 `description` 后，该默认列集合应包含 `description`。

建议默认列为：

- `file.path`
- `file.name`
- `description`
- `file.mtime`
- `file.size`
- `file.tags`

这样做的原因：

- 与 `note resolve` 一样，为 Agent 提供最低成本的语义识别信息
- 减少表达式模式下额外手写 `SELECT file.name, description ...` 的需要
- 保持 `description` 成为面向 Agent 的默认可见字段，而不是隐藏在 frontmatter 中

### 4.5 与单命中结果的关系

引入 `description` 后，Agent 对 `note resolve` 的单命中结果不应只看名称命中方式，还应结合 `description` 做一次低成本语义复核。

处理规则应调整为：

- `alias` 单命中 **仍然不是高置信直接命中**
- `exact` 单命中也**不等于必然可直接复用**；如果 `description` 与当前目标明显不符，应考虑这是“同名但不同语义”的情况
- Agent 应先结合：
  - `type`
  - `description`
  - 当前对话上下文
  做轻量判断
- 若 `description` 明显不符合当前要找的对象，即使是 `exact`，也应优先考虑：
  - 新建一个新 note
  - 或继续追问用户确认是否复用现有 note
- 若仍不足以确认，再执行 `markbase note render <resolved-note-name>` 或询问用户

也就是说，`description` 的目标不是让 `alias` 或 `exact` 变成“自动通过”，而是让单命中结果都变得“更容易低成本判断”。

### 4.6 模板读取归一化

为避免 `template describe`、`note new -t` 等命令各自实现模板兜底逻辑，所有“读取模板内容”的实现应共享一个统一的模板读取/归一化入口。

该入口在内存中对模板做归一化，不直接修改模板文件，并至少保证以下行为：

1. **Schema 层归一化**
   - 若 `_schema.required` 未包含 `description`，自动补入
   - 若 `_schema.properties.description` 不存在，自动补入如下定义：

```yaml
description:
  type: text
  description: 一句话说明这个 note 是什么
```

2. **实例 frontmatter 层归一化**
   - 若模板 outer frontmatter 未显式提供 `description`，系统自动为实例骨架补上该字段

这意味着模板支持三层保障：

1. 模板作者显式声明 `description`
2. 模板读取归一化逻辑自动补齐 `description`
3. `note verify` 的全局规则继续检查最终实例是否存在有效 `description`

---

## 5. 用户与 Agent 可见行为变化

### 5.1 `note resolve` 输出扩展

当前 `resolve` 结果中每个 match 包含：

- `name`
- `path`
- `type`
- `matched_by`

变更后新增：

- `description`

示例：

```json
[
  {
    "query": "张总",
    "status": "alias",
    "matches": [
      {
        "name": "张伟-绿米",
        "path": "people/张伟-绿米.md",
        "type": "person",
        "description": "绿米科技 CTO，负责 AI 平台合作",
        "matched_by": "alias"
      }
    ]
  }
]
```

若缺失，则输出：

```json
{
  "description": null
}
```

### 5.2 `note verify` 行为扩展

`note verify` 需要新增一条全局检查：

- 若 note 缺少 `description` 字段，输出 WARN
- 若 `description` 为空字符串或全空白，输出 WARN
- 若 `description` 存在但类型不是字符串，输出 WARN

执行顺序要求：

- **先执行全局 `description` 检查**
- 再执行基于 `templates` / `_schema` 的模板校验

这样可以保证：

- 即使 note 没有 `templates`，也能得到关于 `description` 的明确反馈
- `description` 真正成为 vault-level invariant，而不是模板存在时才生效的约束
- 普通 Markdown note 与模板实例 note 的行为一致

### 5.3 模板要求扩展

所有模板实例最终都必须产出 `description` 字段。

本设计建议三层同时存在：

- 模板层：尽可能在模板中显式保留 `description`
- 读取层：对旧模板做统一归一化，自动补齐 `_schema.required`、`_schema.properties.description` 和实例骨架中的 `description`
- 校验层：无论模板是否写出，都统一给出 WARN

这样既能兼容旧模板改造过程，也能保证最终一致性。

---

## 6. 推荐填写规范

`description` 建议为一句话、最小但可辨识的语义摘要。

### 6.1 推荐特征

- 优先说明“这是什么”而不是“发生了什么细节”
- 长度适中，适合列表和 JSON 输出阅读
- 不要求完美，但应优于仅看文件名

### 6.2 示例

**人物 note**

```yaml
description: 绿米科技 CTO，负责 AI 平台合作
```

**公司 note**

```yaml
description: 智能家居公司，当前为潜在客户
```

**事件 note**

```yaml
description: 2026-03-07 与绿米的 AI Demo 会
```

**临时笔记**

```yaml
description: 临时笔记
```

**日记**

```yaml
description: 个人日记
```

---

## 7. 实施范围

本特性预计影响以下区域：

### 7.1 模板与实例规则

- 所有模板应逐步补齐 `description`
- `template describe`、`note new -t` 等读取模板内容的命令应复用统一模板读取/归一化逻辑
- `note new` 创建的新实例应能自然包含该字段

### 7.2 校验逻辑

- `src/verifier.rs` 增加 `description` 全局检查，并在模板校验前执行
- 相关 CLI 测试需要更新/补充

### 7.3 实体对齐输出

- `src/resolver.rs` 读取并返回 `description`
- `skills/markbase/SKILL.md` 调整 `alias` 决策规则
- `README.md` 更新 `note resolve` 的输出结构说明

### 7.4 默认查询输出

- `src/query/translator.rs` 中的默认 SELECT 列集合需要加入 `description`
- `README.md` 更新 `query` 默认输出列说明

### 7.4 默认查询输出

- `src/query/translator.rs` 中的默认 SELECT 列集合需要加入 `description`
- `README.md` 更新 `query` 默认输出列说明

---

## 8. 向后兼容与迁移

该特性对现有 vault 的影响较大，因为旧 note 很可能普遍缺少 `description`。

### 8.1 兼容性风险

- 旧 note 很可能普遍缺少 `description`
- 旧模板如果未包含 `description`，Agent 在创建时也可能生成不完整实例
- 若不同命令分别做模板兜底，后续行为可能不一致

### 8.2 建议迁移策略

建议先按单阶段软落地推进：

- 代码支持 `resolve` 输出 `description`
- `verify` 对缺失、空白、错误类型统一输出 WARN
- 模板读取入口自动补齐 `description` 相关定义
- 模板逐步补齐显式 `description`

是否在未来升级为 ERROR，可在模板和存量 note 基本迁移完成后单独评估，不作为本阶段前提。

---

## 9. 验收标准

当以下条件全部满足时，可认为该特性完成：

1. `markbase note resolve` 的输出包含 `description`
2. `note verify` 能在模板校验前稳定检查 `description` 缺失、空白或类型错误，并以 WARN 报告
3. 关键模板已补齐 `description`
4. `template describe` 与 `note new -t` 已复用统一模板读取/归一化逻辑
5. `query` 默认输出列已包含 `description`
6. `README.md` 与 `skills/markbase/SKILL.md` 已同步更新
7. 针对以下情况具备测试覆盖：
   - note 有 `description`
   - note 缺少 `description`
   - note 的 `description` 为空
   - note 的 `description` 为全空白
   - note 的 `description` 类型错误
   - `resolve` 输出中含 `description`
   - 表达式模式下 `query` 默认输出包含 `description`
   - `alias` 命中时可读取 `description`
   - 模板缺少 `_schema.required.description` 时可被读取层补齐

---

## 10. 已确认决策

以下问题已在设计讨论中确认：

1. **不**为 `description` 定义更严格的最小有效内容规则；本阶段只检查缺失、空白和类型错误。
2. `note new` 在**无模板场景**下，**需要**自动生成一个默认 `description` 占位值。
3. **不**提供批量迁移工具；旧 vault 的补齐工作由后续人工或 Agent 渐进完成。

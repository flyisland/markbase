# `description` 特性实施计划

**状态：** 待执行  
**日期：** 2026-03-08  
**关联设计：** `spec/description_design.md`

---

## 1. 实施原则

- 该计划是临时执行文档，服务于一次或少数几次开发会话
- 完成后可归档或删除
- 长期保留的信息应回到 `spec/description_design.md`
- 本阶段以 **WARN 软落地** 为准，不引入 ERROR 阻断
- 已确认：无模板 `note new` 需要自动生成默认 `description` 占位值
- 已确认：不引入更严格的最小有效内容规则，也不提供批量迁移工具

---

## 2. 建议实施顺序

### Step 1：抽取统一模板读取/归一化入口

目标：让所有读取模板内容的命令共享一套 `description` 兜底逻辑。

涉及修改：

- 新增共享模板读取模块（文件名待定，如 `src/template.rs`）
- `src/describe.rs`
- `src/creator.rs`

归一化要求：

- 若 `_schema.required` 未包含 `description`，自动补入
- 若 `_schema.properties.description` 缺失，自动补入 `type: text`
- 若模板 outer frontmatter 缺少 `description`，自动为实例骨架补上该字段
- 归一化只发生在内存中，不直接回写模板文件

验收点：

- `template describe` 与 `note new -t` 均复用同一入口
- 旧模板即使未声明 `description`，读取结果也具备统一语义

---

### Step 2：扩展 `resolve` 输出

目标：让 `markbase note resolve` 返回稳定的 `description` 字段。

涉及修改：

- `src/resolver.rs`
- 对应单元测试与 CLI 测试

验收点：

- `exact` / `alias` / `multiple` 的 match 中都可见 `description`
- 无 `description` 时输出显式 `null`
- 字段缺失时不省略 key

---

### Step 3：扩展 `query` 默认输出列

目标：让空输入与表达式模式下的默认查询结果直接包含 `description`。

涉及修改：

- `src/query/translator.rs`
- 相关单元测试
- `README.md`

验收点：

- 默认 SELECT 列包含 `description`
- 表达式模式下无需显式 `SELECT` 也能看到 `description`
- 相关文档说明与真实行为一致

---

### Step 4：为 `verify` 添加全局检查

目标：无论模板是否显式声明，所有 Markdown note 都能获得 `description` 的一致反馈。

涉及修改：

- `src/verifier.rs`
- `tests/cli_note.rs`
- 如有必要，补充相关设计说明文档

建议规则：

- 字段缺失 → WARN
- 字段存在但为空字符串或全空白 → WARN
- 字段存在且为非字符串 → WARN
- 全局检查先于模板校验执行

验收点：

- 即使 note 没有 `templates`，也能收到关于 `description` 的 WARN
- 缺失、空值、空白值、错误类型均可被稳定识别

---

### Step 5：更新模板

目标：让新创建 note 更自然地满足 `description` 约束，并逐步减少 WARN。

涉及修改：

- `templates/` 下模板文件
- 必要时更新模板说明文档

建议做法：

- 所有模板 frontmatter 都显式补上 `description`
- 所有模板 `_schema.required` 都显式包含 `description`
- 所有模板 `_schema.properties` 都显式定义 `description`

建议示例：

```yaml
description: ""

_schema:
  required: [description]
  properties:
    description:
      type: text
      description: 一句话说明这个 note 是什么
```

说明：

- 空字符串在本阶段是允许创建、但会触发 WARN 的过渡状态
- 长期目标仍然是由 Agent 或用户填入真实内容

---

### Step 6：实现 `note new` 无模板默认值

目标：让无模板创建的 Markdown note 也具备最小语义摘要字段。

涉及修改：

- `src/creator.rs`
- 对应测试
- `README.md`

建议规则：

- 当 `note new` 未指定模板时，自动生成默认 `description` 占位值
- 默认值文案可采用 `临时笔记`
- 该行为仅针对无模板场景，不影响模板实例的字段生成逻辑

验收点：

- `markbase note new my-note` 生成的 frontmatter 包含 `description`
- 默认值行为在 README 中有明确说明

---

### Step 7：同步文档与 Agent Skill

涉及修改：

- `README.md`
- `skills/markbase/SKILL.md`
- 必要时 `AGENTS.md`
- 如有需要，`spec/template_schema.md`

需要更新的内容：

- `resolve` 输出结构新增 `description`
- `description` 缺失时输出 `null`，而不是省略字段
- `alias` 命中时，先结合 `description` / `type` / 上下文判断
- `exact` 单命中时，也要结合 `description` 做语义复核；若明显不符，应考虑新建 note 或继续确认
- 所有 Markdown note 统一要求 `description`
- 模板读取命令共享归一化逻辑
- `query` 默认输出列新增 `description`
- 无模板 `note new` 会自动补默认 `description`

---

### Step 8：测试与验证

至少执行：

```bash
cargo fmt --check
cargo test
cargo clippy -- -D warnings
```

建议新增/确认的测试：

- `resolve` 输出带 `description`
- `exact` 单命中但 `description` 明显不符时，文档与 Skill 会提示不要直接复用
- `resolve` 在缺失 `description` 时输出 `null`
- 表达式模式下 `query` 默认输出包含 `description`
- 缺失 `description` 的 verify WARN 行为
- 空字符串 `description` 的 verify WARN 行为
- 空白字符串 `description` 的 verify WARN 行为
- `description` 非字符串的 verify WARN 行为
- 模板缺少 `_schema.required.description` 时的归一化行为
- 模板缺少 outer `description` 时，`note new -t` 生成的实例包含该字段
- 无模板 `note new` 生成默认 `description`

---

## 3. 推荐拆分方式

为降低 review 风险，建议拆成四次提交：

### Commit 1：模板读取归一化

范围：

- 共享模板读取模块
- `src/describe.rs`
- `src/creator.rs`
- 相关测试

提交目标：

- 统一模板读取入口
- 实现 `description` 归一化兜底

### Commit 2：默认输出增强

范围：

- `src/resolver.rs`
- `src/query/translator.rs`
- 相关测试
- `README.md`

提交目标：

- `resolve` 支持 `description`
- `query` 默认输出加入 `description`
- 文档同步默认输出变化

### Commit 3：校验与无模板创建

范围：

- `src/verifier.rs`
- `src/creator.rs`
- `tests/cli_note.rs`
- `README.md`
- `skills/markbase/SKILL.md`

提交目标：

- 完成 WARN 级别校验逻辑
- 无模板 `note new` 自动补默认 `description`
- 文档同步

### Commit 4：模板显式迁移

范围：

- `templates/`
- 必要的模板说明文档

提交目标：

- 模板全部显式补齐 `description`
- 降低新实例创建后的 WARN 率

---

## 4. 执行清单

- [ ] 抽取统一模板读取/归一化入口
- [ ] 扩展 `resolve` 输出，增加 `description`
- [ ] 约定 `description` 缺失时输出显式 `null`
- [ ] 调整 `query` 默认输出列，加入 `description`
- [ ] 为 `verify` 增加 `description` 全局 WARN 检查
- [ ] 调整 `verify` 执行顺序：先全局检查，后模板校验
- [ ] 为无模板 `note new` 自动补默认 `description`
- [ ] 增加或更新相关测试
- [ ] 更新 `README.md`
- [ ] 更新 `skills/markbase/SKILL.md`
- [ ] 视需要更新 `spec/template_schema.md`
- [ ] 批量补齐模板中的 `description`
- [ ] 运行 `cargo fmt --check`
- [ ] 运行 `cargo test`
- [ ] 运行 `cargo clippy -- -D warnings`

---

## 5. 建议下一会话开场动作

下一会话可以直接从以下步骤开始：

1. 新建共享模板读取模块
2. 先让 `template describe` 和 `note new -t` 共用它
3. 再实现 `src/resolver.rs` 的 `description` 输出
4. 再把 `query` 默认输出列加上 `description`
5. 再实现 `src/verifier.rs` 的全局 WARN 检查和无模板默认值
6. 最后统一补文档和模板

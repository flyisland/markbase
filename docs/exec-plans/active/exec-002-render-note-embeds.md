---
id: exec-002
title: "Render Note Embeds"
status: active
design-doc: design-002
parallel_safe_verified: false
---

## Goal

为 `markbase note render` 增加 Markdown note embed 的正式实现路径，使 `![[note]]` 能展开为目标 note 的 render 结果，并与当前 active 设计文档保持一致：

- 只输出被嵌入 note 的 body，不输出 frontmatter
- inline note embed 按块插入，拆分前后文本为独立行
- 支持多层 note / `.base` 递归展开
- 用显式 cycle guard 阻止循环嵌入导致的无限递归
- 保持 direct `.base` render 的既有语义不变

这个计划的目标不是单纯“让 note embed 能工作”，而是把 note embed render 语义纳入 renderer 的稳定 contract，并补齐 README 与回归测试，让后续改动有明确验收边界。

## Phases

### Phase 1: 基础 note embed 语义

- [ ] task-006: 为 renderer 增加 whole-note embed 展开，明确只输出 body 且 inline 按块拆行

### Phase 2: 递归与循环保护

- [ ] task-007: 为 note embed render 增加递归展开和 cycle guard

### Phase 3: 对外合约与回归

- [ ] task-008: 同步 README 与 render 回归测试，使实现、文档和验收保持一致

## Execution Mode

按顺序串行执行，不并行。

原因：

- `task-006` 定义 note embed 的基础输出形态，是后续递归行为的前置条件
- `task-007` 依赖 `task-006` 已经建立 note embed 的 render 入口，否则 cycle guard 没有稳定挂载点
- `task-008` 必须在实现完成后同步 README 和最终测试断言，避免文档提前锁死尚未收敛的实现细节

## Dependencies

task-006 -> task-007 -> task-008

## Decision Log

### 2026-03-14: 将 note embed render 拆成多任务执行，而不是单一 task

原因：这次改动包含两类风险明显不同的行为。

- 基础 note embed 展开关注输出边界：只渲染 body、不输出 frontmatter、inline 拆行
- 递归与 cycle guard 关注执行安全性：多层展开、软失败 warning、placeholder 占位

把它们拆开后，测试和代码评审都更容易定位问题，也更符合当前 `exec plan + task spec` 体系的使用方式。

### 2026-03-14: 将 README/回归收口独立为最后一个 task

原因：README 和最终回归测试必须反映已经落地的最终行为，而不是中途方案。把文档和测试收口放在最后，能避免在递归行为和失败输出尚未稳定时反复改对外合同。

## Progress Notes

- 2026-03-14: 建立 `exec-002` 与配套 `task-006` 至 `task-008`，作为 note embed render 功能的实现入口
- 2026-03-14: active 设计文档 `design-002-render.md` 与 `design-001-links-and-embeds.md` 已先行补齐 note embed render 合同，后续实现应以此为准

## Definition of Done

`exec-002` 只有在以下条件全部满足时才算完成：

1. `![[note]]` 和 `![[note|Alias]]` 在 `note render` 中都能展开为目标 note 的 body
2. 被嵌入 note 的 frontmatter 不会出现在 render 输出中
3. inline note embed 会按设计拆成前文、展开 body、后文三段，而不是把展开内容塞回原行
4. 嵌入 note 内部的 note embed 和 `.base` embed 都会继续展开
5. 遇到循环嵌入时，renderer 会输出文档约定的 warning 和 placeholder，并继续渲染剩余内容
6. 嵌入 note 缺失或读取失败时，renderer 会输出文档约定的 warning 和 placeholder，并继续渲染剩余内容
7. 嵌入 note 内部的 `.base` embed 以当前正在展开的 embedded note 作为 `this` 上下文，而不是原始 top-level render target
8. direct `.base` render 行为不因本计划发生语义变化
9. README、设计文档、回归测试与实现一致
10. `cargo test`、`cargo clippy -- -D warnings`、`cargo fmt --check` 通过

## Blocking Rules

执行过程中如果遇到以下情况，不要自行扩展语义，必须先回到本计划和 `design-002` / `design-001` 对齐：

- 想把 `![[note#Heading]]` 或 `![[note#^blockid]]` 一并作为可执行 note embed
- 想让 embedded note 输出 frontmatter、标题包装、来源包装，或其他额外结构
- 想把循环嵌入改为 hard error 并中止整个 render
- 想顺手改变 direct `.base` render 的入口或行为
- 想绕开共享 `MarkdownBody` 扫描规则，单独为 renderer 增加另一套 embed 探测逻辑

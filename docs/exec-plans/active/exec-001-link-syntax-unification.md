---
id: exec-001
title: "Link Syntax Unification"
status: active
design-doc: design-001
parallel_safe_verified: false
---

## Goal

统一 markbase 对 Obsidian `[[...]]` / `![[...]]` 的解析语义，消除 `extractor`、`renamer`、`verifier`、`renderer` 之间的实现漂移，并补齐当前最符合应用场景的缺口：escaped pipe、`.md` + anchor/alias、`.base#View` 渲染、以及 code context 排除。

这个计划的目标不是“修一批 regex 边界 bug”，而是建立一个**共享语义层**，让后续功能以同一套 token 和 normalization contract 为基础。

## Phases

### Phase 1: 语义基础层

- [ ] task-001: 建立共享 link/embed 解析器，替代多处直接 regex 解释
- [ ] task-002: 将 extractor 和 verifier 切换到共享解析语义

### Phase 2: 写路径与渲染路径

- [ ] task-003: 将 renamer 切换为基于 token span 的精确改写
- [ ] task-004: 为 renderer 增加 `.base#View` 支持，并与共享解析保持一致

### Phase 3: 合约与回归

- [ ] task-005: 同步设计文档、README、Agent 入口文档，并补齐跨模块回归测试

## Execution Mode

按顺序串行执行，不并行。

原因：

- `task-001` 决定共享解析层 API，是后续所有任务的前置条件
- `task-003` 和 `task-004` 都依赖 `src/link_syntax.rs`，并且都会改 `tests/cli_note.rs`
- `task-005` 必须在实现完成后再同步最终对外合约

## Dependencies

task-001 -> task-002 -> task-003 -> task-005

task-001 -> task-004 -> task-005

## Decision Log

### 2026-03-14: 采用共享解析层，而不是继续局部 regex 修补

原因：当前 link/embed 语义由 `src/extractor.rs`、`src/renamer.rs`、`src/verifier.rs`、`src/renderer/mod.rs` 分散解释。继续局部修补会保留语义漂移风险，也无法稳定解决 escaped pipe、code context 排除和 `.base#View` 这类跨模块问题。

### 2026-03-14: 旧 `spec/*.md` 作为 legacy 设计文档归档

原因：这些文件实际承担的是设计说明而非验证型 spec 职责。为避免与新的 `specs/*.spec` 任务合约体系混淆，旧文件迁移到 `docs/design-docs/legacy/`，当前任务体系改为 `docs/exec-plans/` + `specs/active/`。

## Progress Notes

- 2026-03-14: 建立 `exec-001` 和配套 `task-001` 至 `task-005`，作为后续实现与验收入口
- 2026-03-14: 归档旧 `spec/*.md` 到 `docs/design-docs/legacy/`，保留历史设计上下文

## Definition of Done

`exec-001` 只有在以下条件全部满足时才算完成：

1. `src/link_syntax.rs` 成为 link/embed 解析的唯一共享语义入口
2. `src/extractor.rs`、`src/renamer.rs`、`src/verifier.rs`、`src/renderer/mod.rs` 都不再各自定义另一套 target 解释规则
3. escaped pipe、`.md#Heading|Alias`、body code context 排除、`.base#View` 四类行为都有回归测试
4. `docs/design-docs/design-001-links-and-embeds.md` 与实现一致，不再把这些行为标成“已知限制”
5. `cargo test`、`cargo clippy -- -D warnings`、`cargo fmt --check` 通过

## Blocking Rules

执行过程中如果遇到以下情况，不要自行发明新语义，必须先回到本计划和 `design-001` 对齐：

- 解析规则与当前文档不一致
- 需要引入 Markdown inline link / image 语法支持
- 需要改变 `links` / `embeds` 的存储模型
- 需要放宽 `.base` 渲染范围到 blockquote/list/callout 前缀行

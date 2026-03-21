---
id: exec-005
title: "Web Note View"
status: completed
design-doc: design-003
parallel_safe_verified: false
---

## Goal

按 [design-003-web-note-view.md](../../design-docs/implemented/design-003-web-note-view.md) 为 markbase 建立第一版 web note view 交付路径，并先修正当前 renderer 与 active 设计不一致的 quote-container 行为。

这个计划的目标不是笼统地“加一个 web server”，而是按现有模块边界分阶段交付以下能力：

- 修正 `note render` 中 callout / blockquote 内 live embed 展开会破坏容器结构的问题，使实现与 active render / web 设计一致
- 增加基于 `file.path` 的 canonical web route resolution，同时保持 markbase 内部 note identity 仍然按 note name 工作
- 为 renderer 增加 web 输出模式，复用既有 whole-note embed / `.base` 语义，但输出 docsify/marked-renderable Markdown
- 增加服务端 OFM normalization，将浏览器无法直接理解的 vault-aware 语义在服务端完成转换
- 增加 `markbase web serve` 与 `markbase web get <canonical-url>` 的第一版对外接口
- 用 README、ARCHITECTURE 和回归测试把新的对外合同固定下来

## Phases

### Phase 1: 渲染语义纠偏

- [x] task-0014: 修正 quote-container preservation，使 callout / blockquote 中的 live note embed 与 `.base` embed 展开后逐行保留 quote prefix、blank line 和 nested depth，list item 仍保持 literal output

### Phase 2: Canonical Web Routing

- [x] task-0015: 建立按 `file.path` 解析的 canonical web route 和 request-scoped index / DB lifecycle，供 `web serve` 与 `web get` 共享

### Phase 3: Web Render Mode

- [x] task-0016: 在 renderer 中增加 web 输出模式，复用既有 render 语义并将 `.base` 默认输出切换为 Markdown table

### Phase 4: OFM Normalization And Public Interface

- [x] task-0017: 增加遵守 Markdown body/code-context 边界的服务端 OFM normalization、resource delivery / content-type 合同、`web serve` / `web get` 对外接口、README / ARCHITECTURE 更新和最终验收测试

## Execution Mode

按顺序串行执行，不并行。

原因：

- `task-0014` 是语义纠偏，不先收敛 quote-container preservation，后续 web 输出只能建立在错误结构之上
- `task-0015` 负责建立 `web serve` 与 `web get` 的共享请求入口，后续 web mode 和 normalization 都依赖这条路径
- `task-0016` 必须建立在已稳定的 render 语义和 route resolution 上，否则 web 输出模式会和 CLI render 漂移
- `task-0017` 负责最终的服务端 rewrite、对外接口和文档收口，必须等前置语义和共享入口稳定后再锁定

## Dependencies

task-0014 -> task-0015 -> task-0016 -> task-0017

## Decision Log

### 2026-03-16: 将 callout / blockquote 结构修正前置为第一任务

原因：这不是 web server 的局部实现细节，而是当前 renderer 行为已经与 active 设计文档不一致。

- [design-002-render.md](../../design-docs/implemented/design-002-render.md) 已将 quote-container preservation 定义为 render contract
- [design-003-web-note-view.md](../../design-docs/implemented/design-003-web-note-view.md) 将 callouts 视为 P0，并明确容器保留是服务端责任
- 如果把修正留到 web 层做补丁，会造成 CLI render 与 web render 拥有两套 embed 语义

因此，必须先在 renderer 层纠偏，再让 web 交付复用这套语义。

### 2026-03-16: 将 web 路由与 `web serve` / `web get` 共享核心拆成独立阶段

原因：设计文档明确要求 `web get <canonical-url>` 返回与 `web serve` 对同一路径相同的 Markdown body。

如果 server 和 CLI inspection helper 分别实现自己的路径解析、索引刷新和 note/resource 判定逻辑，行为很容易漂移。把 canonical route resolution 和 request-scoped lifecycle 先独立出来，后续两条接口都复用同一个核心，测试也更容易覆盖。

### 2026-03-16: 将 web 渲染模式放在 OFM normalization 之前

原因：`design-003` 不是要在现有 CLI stdout 文本上做一次字符串替换，而是要求 renderer 提供一种新的 web-mode 输出 contract：

- note 响应返回纯 Markdown 文本
- `.base` 默认输出 Markdown table，而不是 CLI 默认的 fenced JSON
- whole-note embed、`.base` expansion、soft-failure placeholder、quote-container preservation 仍沿用既有 render 语义

只有先把 web-mode 输出边界收敛，后续 OFM normalization 才有稳定输入。

## Progress Notes

- 2026-03-16: 建立 `exec-005` 初稿，明确这是一个跨 renderer、route resolution、HTTP surface 和文档合同的串行交付
- 2026-03-16: 将 quote-container preservation 修正前置为 `task-0014`，作为整个 web note view 的基础前置条件
- 2026-03-21: rebase `main` 后迁移到受控文档体系，并将 task 编号重排为 `task-0014` 至 `task-0017`
- 2026-03-21: 验证 `task-0014` 对应的 renderer 行为与测试已落地，完成状态迁移，后续进入 `task-0015`
- 2026-03-21: 完成 canonical routing、web render mode、OFM normalization、HTTP/resource contract 与文档收口，执行计划归档

## Definition of Done

`exec-005` 只有在以下条件全部满足时才算完成：

1. `note render` 在 callout / blockquote 中展开 live note embed 与 `.base` embed 时，不再破坏 quote-container 结构
2. quote-container preservation 保留每一行的 quote prefix、blank line 和 nested quote depth
3. list item 中的 live note embed 与 live `.base` embed 仍保持 literal output，不被错误展开
4. web 路由按 `file.path` 解析，内部 render 仍按 `file.name` / note name 复用既有语义
5. 服务端对外输出的 note 响应为 docsify/marked-renderable Markdown，而不是 HTML shell
6. web mode 下 `.base` 默认输出 Markdown table，而不是 CLI 默认 fenced JSON
7. `[[...]]` 被重写为 canonical path-based browser URL，且不会输出仅由 note name 构成的 bare relative href
8. server-emitted URL 会按浏览器 URL 规则对 `file.path` 做 percent-encoding，包括空格、非 ASCII、`#`、`?` 等需要编码的字符
9. heading wikilink 和 block wikilink 在 v1 按 active design 生成稳定的显示文本降级，而不会声称稳定 fragment：
   - `[[note#Heading]]` 无 alias 时显示为 `note > Heading`
   - `[[note#^blockid]]` 无 alias 时显示为 `note`
   - 两者有 alias 时都优先显示 alias
10. OFM normalization 只作用于普通 Markdown body content；fenced code block 与 inline code span 中的 `[[...]]`、`![[...]]`、`%%comment%%` 等示例文本保持 literal output
11. 非 Markdown `![[...]]` 资源 embed 被重写为标准 Markdown image 或 link，并能按 canonical route 获取
12. canonical resource route 对 attachment 返回原始 bytes，且响应 `Content-Type` 与资源类型一致
13. `%%comment%%` 被从 web 输出中移除
14. unresolved wikilink 和 unresolved resource embed 在 v1 仍保持 literal source text
15. selector-based note embeds 和 block-target note embeds 在 v1 仍保持 literal output
16. `markbase web serve` 的 v1 public CLI surface、bind defaults 和 override flags 在设计、README、测试中被明确锁定
17. `markbase web get <canonical-url>` 对 note target 返回与 `markbase web serve` 相同的 Markdown body
18. `markbase web get <canonical-url>` 不会流式输出 binary resource，而是按设计返回解释性失败
19. HTTP route miss 返回 `404 Not Found`，不可解码 HTTP 路径返回 `400 Bad Request`
20. `markbase web get <canonical-url>` 对 miss/bad path 返回与 route-resolution contract 一致的 CLI failure，但不要求复用 HTTP 状态码字面输出
21. README、ARCHITECTURE、相关设计文档和回归测试与最终实现一致
22. `cargo test`、`cargo clippy -- -D warnings`、`cargo fmt --check` 通过

## Blocking Rules

执行过程中如果遇到以下情况，不要自行扩展语义，必须先回到本计划和 active design 对齐：

- 想把 path-based web URL 反向扩展成 markbase 内部 note-facing identity
- 想在 web 层重新实现一套独立的 link / embed parser，而不是复用 `src/link_syntax.rs`
- 想把 docsify shell、HTML entrypoint 或前端主题一起纳入 markbase v1 交付
- 想顺手支持 `![[note#Heading]]`、`![[note#^blockid]]`、block reference rendering、footnotes、math / LaTeX
- 想让 unresolved wikilink 或 unresolved resource embed 在 v1 自动降级为猜测性的 clickable href
- 想让 `web get` 与 `web serve` 走两套不同的 route resolution / render pipeline
- 想在 web mode 中重新定义 whole-note embed、`.base` expansion 或 soft-failure placeholder 的核心语义，而不是复用 active render contract

---
id: task-0017
title: "完成 OFM normalization 与 web 对外接口"
status: active
exec-plan: exec-005
phase: 4
boundaries:
  allowed:
    - "src/main.rs"
    - "src/web/**"
    - "src/renderer/**"
    - "README.md"
    - "ARCHITECTURE.md"
    - "docs/design-docs/candidate/design-003-web-note-view.md"
    - "tests/cli_web.rs"
    - "tests/cli_note.rs"
    - "tests/common/**"
  forbidden_patterns:
    - "specs/**"
    - "src/query/**"
completion_criteria:
  - id: "cc-001"
    scenario: "wikilink 被重写为 canonical path-based browser URL"
    test: "test_web_output_rewrites_wikilinks_to_canonical_routes"
  - id: "cc-002"
    scenario: "server-emitted URL 对 `file.path` 执行 percent-encoding"
    test: "test_web_output_percent_encodes_emitted_urls"
  - id: "cc-003"
    scenario: "heading/block wikilink 采用 active design 规定的显示文本降级"
    test: "test_web_output_uses_design_contract_link_text_for_heading_and_block_links"
  - id: "cc-004"
    scenario: "资源 embed 被重写为标准 Markdown image 或 link"
    test: "test_web_output_rewrites_resource_embeds"
  - id: "cc-005"
    scenario: "comments 被移除，deferred syntax 继续 literal passthrough"
    test: "test_web_output_removes_comments_and_preserves_deferred_syntax"
  - id: "cc-006"
    scenario: "`web get` 对 note target 返回与 `web serve` 相同的 Markdown body"
    test: "test_web_get_matches_web_serve_for_note_targets"
  - id: "cc-007"
    scenario: "`web get` 对 binary resource 拒绝流式输出并返回解释性失败"
    test: "test_web_get_refuses_binary_resource_targets"
  - id: "cc-008"
    scenario: "README 与 ARCHITECTURE 对 web interface 的合同与实现一致"
    test: "test_web_interface_behavior_matches_docs"
---

## Intent

在 canonical routing 和 web render mode 已建立后，完成 `design-003` 定义的服务端 OFM normalization，并交付第一版 `markbase web serve` / `markbase web get <canonical-url>` 对外接口、文档和回归测试。

这个任务是最终收口任务，负责把服务端 rewrite、CLI surface、HTTP 合同和文档统一起来，但不再重新定义前面任务已经收敛的 renderer 核心语义。

## Decisions

- live wikilink 在最终 web 输出中必须被重写为 canonical path-based browser URL，不能输出 bare relative href note name
- server-emitted URL 必须对 `file.path` 执行 percent-encoding，包括空格、非 ASCII、`#`、`?` 等需要编码的字符
- `[[note#Heading]]` 与 `[[note#^blockid]]` 在 v1 只链接到 canonical note route，不声称稳定 fragment；显示文本按 active design 降级：
  - heading link 有 alias 时显示 alias，无 alias 时显示 `note > Heading`
  - block link 有 alias 时显示 alias，无 alias 时显示 `note`
- resource embed rewrite 仅覆盖 v1 已定义的非 Markdown、非 `.base` 资源：图片转 Markdown image，PDF 与其他附件转标准 link
- unresolved wikilink 和 unresolved resource embed 在 v1 继续保持 literal source text
- `%%comment%%` 在 web 输出中被移除；`==highlight==`、Mermaid、footnotes、math、selector-based note embed、block-target note embed 继续按 design-003 的 v1 边界处理
- `markbase web serve` 返回 note Markdown 或 resource bytes；markbase 不负责 docsify shell 或 HTML entrypoint
- `markbase web get <canonical-url>` 仅用于 inspection：note target 输出最终 Markdown body，binary resource target 返回解释性失败而不是流式输出 bytes
- README 只描述用户可见命令和行为；ARCHITECTURE 负责补充 web layer 的系统边界与请求生命周期角色
- 若最终实现与 `design-003` 局部表述存在冲突，应在同一任务中同步回写文档与测试，保持单一合同

## Boundaries

### Allowed Changes

- src/main.rs
- src/web/**
- src/renderer/**
- README.md
- ARCHITECTURE.md
- docs/design-docs/candidate/design-003-web-note-view.md
- tests/cli_web.rs
- tests/cli_note.rs
- tests/common/**

### Forbidden

- 不得把 docsify shell、主题、静态页面打包纳入 markbase v1
- 不得顺手支持 selector-based note embed、block reference、footnotes 或 math
- 不得把 unresolved link/resource 自动降级成猜测性的 clickable href
- 不得让 `web get` 绕过 `web serve` 的核心处理路径
- 不得通过修改文档弱化已经落地的对外 contract

## Completion Criteria

场景: wikilink 被重写为 canonical path-based browser URL
测试: test_web_output_rewrites_wikilinks_to_canonical_routes
假设 note body 中存在 `[[note-a]]`、`[[note-a|Alias]]`、`[[note-a#Heading]]`
当   通过最终 web path 输出
那么 这些 live wikilink 被重写为指向 canonical route 的标准 Markdown link
并且 不会输出仅由 note name 构成的 bare relative href

场景: server-emitted URL 对 `file.path` 执行 percent-encoding
测试: test_web_output_percent_encodes_emitted_urls
假设 canonical target 的 `file.path` 含有空格、非 ASCII、`#` 或 `?`
当   最终 web 输出生成 note link 或 resource URL
那么 输出 href 使用 percent-encoded URL
并且 route identity 仍对应原始 decoded `file.path`

场景: heading/block wikilink 采用 active design 规定的显示文本降级
测试: test_web_output_uses_design_contract_link_text_for_heading_and_block_links
假设 note body 中存在 `[[note-a#Heading]]`、`[[note-a#Heading|Alias]]`、`[[note-a#^blockid]]`、`[[note-a#^blockid|Alias]]`
当   通过最终 web path 输出
那么 heading link 无 alias 时显示 `note-a > Heading`
并且 heading link 有 alias 时显示 `Alias`
并且 block link 无 alias 时显示 `note-a`
并且 block link 有 alias 时显示 `Alias`
并且 这些链接都只指向 canonical note route，不声称稳定 fragment

场景: 资源 embed 被重写为标准 Markdown image 或 link
测试: test_web_output_rewrites_resource_embeds
假设 note body 中存在 `![[image.png]]`、`![[file.pdf]]` 与其他附件 embed
当   通过最终 web path 输出
那么 图片被重写为 Markdown image
并且 PDF 与其他附件被重写为标准 Markdown link

场景: comments 被移除，deferred syntax 继续 literal passthrough
测试: test_web_output_removes_comments_and_preserves_deferred_syntax
假设 note body 中同时存在 `%%comment%%`、`![[note#Heading]]`、`![[note#^blockid]]`
当   通过最终 web path 输出
那么 comments 被移除
并且 deferred selector/block syntax 继续按字面输出

场景: `web get` 对 note target 返回与 `web serve` 相同的 Markdown body
测试: test_web_get_matches_web_serve_for_note_targets
假设 某 canonical note URL 可被 `web serve` 正常处理
当   对同一路径执行 `markbase web get <canonical-url>`
那么 输出的 Markdown body 与 `web serve` 对该路径返回的 body 一致

场景: `web get` 对 binary resource 拒绝流式输出并返回解释性失败
测试: test_web_get_refuses_binary_resource_targets
假设 canonical URL 指向图片或 PDF 等 binary resource
当   执行 `markbase web get <canonical-url>`
那么 命令不会流式输出原始 bytes
并且 返回设计约定的解释性失败

场景: README 与 ARCHITECTURE 对 web interface 的合同与实现一致
测试: test_web_interface_behavior_matches_docs
假设 README 已记录 `web serve` / `web get` 的用户可见行为，ARCHITECTURE 已记录 web layer 边界
当   执行对应 CLI / integration 回归测试
那么 文档描述、实现行为与 active design 保持一致

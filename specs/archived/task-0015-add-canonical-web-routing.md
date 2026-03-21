---
id: task-0015
title: "建立 canonical web 路由与请求级生命周期"
status: completed
exec-plan: exec-005
phase: 2
boundaries:
  allowed:
    - "src/main.rs"
    - "src/web/**"
    - "src/db.rs"
    - "README.md"
    - "ARCHITECTURE.md"
    - "tests/cli_web.rs"
    - "tests/common/**"
  forbidden_patterns:
    - "specs/**"
    - "src/renderer/**"
completion_criteria:
  - id: "cc-001"
    scenario: "canonical note route 按 `file.path` 解析到内部 note name"
    test: "test_web_route_resolves_note_path_to_internal_note_name"
  - id: "cc-002"
    scenario: "请求路径先 decode 再按 `file.path` 匹配"
    test: "test_web_route_matches_decoded_file_path"
  - id: "cc-003"
    scenario: "每次请求在 route resolution 前刷新索引"
    test: "test_web_request_refreshes_index_before_route_resolution"
  - id: "cc-004"
    scenario: "请求结束后关闭 request-scoped DB handle"
    test: "test_web_request_closes_request_scoped_db_handle"
  - id: "cc-005"
    scenario: "HTTP route miss 与不可解码 HTTP 路径分别返回 404 和 400"
    test: "test_web_http_route_returns_404_for_miss_and_400_for_bad_path"
  - id: "cc-006"
    scenario: "`web get` 对 miss/bad path 返回 CLI failure，但不绑定 HTTP 状态码字面输出"
    test: "test_web_get_returns_cli_failure_for_miss_and_bad_path"
---

## Intent

建立 `design-003` 要求的 canonical web route resolution 和 request-scoped index / DB lifecycle，作为 `markbase web serve` 与 `markbase web get <canonical-url>` 共享的请求入口。

这个任务的重点是路径身份、索引刷新和请求生命周期，不负责 web-mode Markdown 输出形态，也不负责 OFM normalization。

## Decisions

- web note 与 resource 的 canonical logical identity 都是 vault-relative `file.path`
- 路由匹配前先对 incoming HTTP path 或 CLI canonical URL 做一次 URL decode，再与 indexed `file.path` 匹配
- server-emitted URL 的 percent-encoding 属于后续输出问题；本任务只建立 decode-and-match contract
- note route 解析到 row 后，后续 render 仍按既有 note name / `file.name` 身份复用 renderer
- resource route 解析后只返回资源定位结果，不在本任务内定义资源内容 rewrite
- 每次请求在 route resolution 前都先走现有 incremental indexing path，不引入第二套索引刷新机制
- DB connection lifetime 为 request-scoped；请求完成后关闭 handle，不保留长连接
- `web serve` 与 `web get` 必须复用同一套 route resolution 和 request lifecycle 核心
- HTTP 侧的 404 与 400 是 route contract，不通过 in-band Markdown warning 表示
- `web get` 复用相同的 route resolution 结果，但它是 CLI inspection helper；miss/bad path 只要求返回与 route-resolution contract 一致的 CLI failure，不要求复用 HTTP 状态码字面输出
- 如果该阶段已经引入可直接调用的 `web serve` / `web get` CLI surface，则必须同步更新 README / ARCHITECTURE 记录当前已落地的 request lifecycle 与 route contract；若仅落共享内部入口和测试钩子，则 public interface 细节继续由 `task-0017` 锁定

## Boundaries

### Allowed Changes

- src/main.rs
- src/web/**
- src/db.rs
- README.md
- ARCHITECTURE.md
- tests/cli_web.rs
- tests/common/**

### Forbidden

- 不得在此任务中引入 web-specific renderer 输出模式
- 不得在此任务中实现 wikilink rewrite 或资源 embed rewrite
- 不得把 `file.path` 扩展为 markbase 内部 note-facing identity
- 不得为 `web get` 和 `web serve` 建两套独立的 route resolution 逻辑
- 不得引入长生命周期的全局 DuckDB 连接作为 v1 contract

## Completion Criteria

场景: canonical note route 按 `file.path` 解析到内部 note name
测试: test_web_route_resolves_note_path_to_internal_note_name
假设 vault 中存在 path 为 `entities/person/张三.md` 的 note
当   请求 canonical path `/entities/person/%E5%BC%A0%E4%B8%89.md`
那么 系统会匹配到该 note 的 `file.path`
并且 后续 render 使用其内部 note name，而不是 path-based note identity

场景: 请求路径先 decode 再按 `file.path` 匹配
测试: test_web_route_matches_decoded_file_path
假设 `file.path` 含有空格、非 ASCII 或保留字符文件名
当   请求对应的 percent-encoded canonical URL
那么 系统先 decode 一次再匹配
并且 匹配依据是 decoded vault-relative `file.path`

场景: 每次请求在 route resolution 前刷新索引
测试: test_web_request_refreshes_index_before_route_resolution
假设 vault 在两次请求之间发生文件变化
当   发起新的 web 请求
那么 系统会先执行 incremental indexing
并且 route resolution 看到的是最新索引状态

场景: 请求结束后关闭 request-scoped DB handle
测试: test_web_request_closes_request_scoped_db_handle
假设 一个请求已完成 note 或 resource 的解析与响应
当   请求处理结束
那么 该请求使用的 DB handle 被关闭
并且 不依赖长生命周期连接维持后续请求

场景: HTTP route miss 与不可解码 HTTP 路径分别返回 404 和 400
测试: test_web_http_route_returns_404_for_miss_and_400_for_bad_path
假设 一个 HTTP 请求路径无法解析到任何 indexed note/resource，另一个 HTTP 请求路径 URL decode 失败
当   分别处理两个 HTTP 请求
那么 前者返回 `404 Not Found`
并且 后者返回 `400 Bad Request`

场景: `web get` 对 miss/bad path 返回 CLI failure，但不绑定 HTTP 状态码字面输出
测试: test_web_get_returns_cli_failure_for_miss_and_bad_path
假设 一个 canonical URL 无法解析到任何 indexed note/resource，另一个 canonical URL 在 decode 时失败
当   分别执行 `markbase web get <canonical-url>`
那么 两者都返回符合 route-resolution contract 的 CLI failure
并且 spec 不要求命令字面输出 HTTP 状态码文本

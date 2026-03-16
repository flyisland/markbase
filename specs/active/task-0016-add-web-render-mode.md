---
id: task-0016
title: "为 renderer 增加 web 输出模式"
status: active
exec-plan: exec-005
phase: 3
boundaries:
  allowed:
    - "src/renderer/**"
    - "src/web/**"
    - "tests/cli_note.rs"
    - "tests/cli_web.rs"
  forbidden_patterns:
    - "specs/**"
    - "src/query/**"
    - "README.md"
    - "ARCHITECTURE.md"
completion_criteria:
  - id: "cc-001"
    scenario: "web mode note 响应输出纯 Markdown 文本"
    test: "test_web_render_mode_returns_plain_markdown_body"
  - id: "cc-002"
    scenario: "web mode 下 `.base` 默认输出 Markdown table"
    test: "test_web_render_mode_base_output_defaults_to_markdown_table"
  - id: "cc-003"
    scenario: "whole-note embed 与 `.base` expansion 在 web mode 中继续生效"
    test: "test_web_render_mode_reuses_recursive_note_and_base_expansion"
  - id: "cc-004"
    scenario: "soft-failure placeholder 与 quote-container preservation 在 web mode 中继续保留"
    test: "test_web_render_mode_preserves_placeholders_and_quote_containers"
---

## Intent

在现有 render 语义基础上增加 web 输出模式，使 note 响应输出 docsify/marked-renderable Markdown，而不是复用 CLI 的默认 stdout 包装格式。

这个任务的重点是输出模式切换和 render contract 复用，不负责 canonical route resolution，也不负责 OFM normalization 规则本身。

## Decisions

- web mode 是 render subsystem 的一种新输出 contract，不是对 CLI 默认输出做字符串后处理
- web mode 继续复用既有 render 语义：whole-note embed expansion、`.base` execution、soft-failure placeholder、quote-container preservation
- web mode note 响应输出纯 Markdown 文本，不输出 HTML shell，不引入前端框架专用包装
- web mode 下 `.base` 默认使用 Markdown table 输出，而不是 CLI 默认的 fenced JSON block
- direct `.base` render 的 CLI 默认输出合同不因 web mode 改变
- web mode 只改变输出形态，不改变 `.base#View` 选择、recursive note render、cycle guard 或 list item exclusion 语义
- web mode 的输入仍是 renderer 语义层结果；后续 OFM normalization 以该模式的输出为稳定输入

## Boundaries

### Allowed Changes

- src/renderer/**
- src/web/**
- tests/cli_note.rs
- tests/cli_web.rs

### Forbidden

- 不得改变 CLI `markbase note render` 的默认 JSON fence 合同
- 不得把 web mode 与 OFM normalization 混为一个不可分离的大函数
- 不得重新定义 whole-note embed、`.base` expansion 或 cycle guard 的语义
- 不得在此任务中实现 canonical URL rewrite
- 不得在此任务中引入 HTML shell、docsify 入口页或主题资源

## Completion Criteria

场景: web mode note 响应输出纯 Markdown 文本
测试: test_web_render_mode_returns_plain_markdown_body
假设 一个 Markdown note 被走 web render path 渲染
当   返回响应体
那么 输出为纯 Markdown 文本
并且 不包含 CLI default `.base` JSON wrapper 或额外 HTML shell

场景: web mode 下 `.base` 默认输出 Markdown table
测试: test_web_render_mode_base_output_defaults_to_markdown_table
假设 note body 中包含 `![[tasks.base]]`
当   通过 web render mode 渲染
那么 `.base` 的结果默认以 Markdown table 形式出现
并且 不再使用 CLI 默认的 fenced JSON block

场景: whole-note embed 与 `.base` expansion 在 web mode 中继续生效
测试: test_web_render_mode_reuses_recursive_note_and_base_expansion
假设 note body 中同时包含 `![[note-a]]` 与 `![[tasks.base]]`
当   通过 web render mode 渲染
那么 embedded note 继续递归展开
并且 `.base` 继续按既有 render contract 执行

场景: soft-failure placeholder 与 quote-container preservation 在 web mode 中继续保留
测试: test_web_render_mode_preserves_placeholders_and_quote_containers
假设 quote container 内的 live embed 缺失、触发 cycle guard 或正常展开
当   通过 web render mode 渲染
那么 placeholder 和正常展开内容都继续保留 quote-container 结构
并且 行为与 renderer 的 active contract 一致

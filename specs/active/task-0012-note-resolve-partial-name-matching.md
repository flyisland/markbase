---
id: task-0012
title: "为 note resolve 增加确定性部分名称匹配"
status: active
boundaries:
  allowed:
    - "src/resolver.rs"
    - "tests/cli_note.rs"
    - "README.md"
    - "docs/design-docs/implemented/design-008-note-resolve.md"
    - "specs/active/task-0012-note-resolve-partial-name-matching.md"
  forbidden_patterns:
    - "src/query/**"
    - "src/db.rs"
    - "src/scanner.rs"
completion_criteria:
  - id: "cc-001"
    scenario: "查询是笔记名称的一部分时，resolve 返回 `name_contains_query`"
    test: "test_note_resolve_name_contains_query_single_match"
  - id: "cc-002"
    scenario: "笔记名称是查询的一部分时，resolve 返回 `query_contains_name`"
    test: "test_note_resolve_query_contains_name_single_match"
  - id: "cc-003"
    scenario: "alias 仍然优先于两类部分名称匹配"
    test: "test_note_resolve_alias_ranks_before_partial_name_matches"
  - id: "cc-004"
    scenario: "同一笔记同时满足多种规则时，只返回一次且采用最高优先级来源"
    test: "test_note_resolve_deduplicates_match_sources_by_priority"
  - id: "cc-005"
    scenario: "README 对 resolve 新状态和匹配来源的说明与实现一致"
    test: "test_note_resolve_behavior_matches_readme_contract"
  - id: "cc-006"
    scenario: "name、alias 和部分名称匹配都不区分大小写"
    test: "test_note_resolve_partial_name_matching_is_case_insensitive"
---

## Intent

让 `markbase note resolve` 在现有 `name` / `alias` 精确匹配之外，支持两种可解释、可排序的 `notes.name` 子串匹配：

- 查询名称是笔记名称的一部分
- 笔记名称是查询名称的一部分

这个任务的目标不是把 `resolve` 扩展成 fuzzy search，而是在不牺牲可预测性的前提下，降低用户和 agent 因“名称写长一点或写短一点”、或者仅仅大小写不同导致的 miss 率。

## Decisions

- 新规则只作用于 `notes.name`，不对 frontmatter `aliases` 做部分匹配
- 匹配优先级固定为：`name` > `alias` > `name_contains_query` > `query_contains_name`
- 若同一 note 同时满足多条规则，只返回一次，`matched_by` 采用最高优先级来源
- 单条命中时，`status` 必须与该条结果的 `matched_by` 对应：
  - `name` -> `exact`
  - `alias` -> `alias`
  - `name_contains_query` -> `name_contains_query`
  - `query_contains_name` -> `query_contains_name`
- 多条命中一律返回 `multiple`，不能因为存在高优先级候选而隐藏其他候选
- 同一优先级内，候选排序固定为：
  1. `abs(length(name) - length(query))` 升序
  2. `name` 升序
  3. `path` 升序
- 该任务不改变输入校验规则，但把 `name` / `alias` / 部分名称匹配统一为大小写不敏感
- 该任务不改变空白敏感语义

## Boundaries

### Allowed Changes

- src/resolver.rs
- tests/cli_note.rs
- README.md
- docs/design-docs/implemented/design-008-note-resolve.md

### Forbidden

- 不得把 `note resolve` 扩展为 fuzzy search 或语义检索
- 不得为 `aliases` 增加部分匹配
- 不得修改数据库 schema，或把匹配状态写回索引
- 不得改变 `note resolve` 的输入校验边界
- 不得通过删除 `multiple` 候选来制造“更智能”的单命中结果
- 不得把大小写不敏感匹配理解为路径匹配、扩展名剥离或 alias 部分匹配

## Completion Criteria

场景: 查询是笔记名称的一部分时，resolve 返回 `name_contains_query`
测试: test_note_resolve_name_contains_query_single_match
假设 vault 中存在名称为 `绿联科技` 的 note，且没有 `绿联` 的精确 name 或 alias 命中
当   执行 `markbase note resolve "绿联"`
那么 JSON `status` 为 `name_contains_query`
并且 第一条 match 的 `matched_by` 为 `name_contains_query`

场景: 笔记名称是查询的一部分时，resolve 返回 `query_contains_name`
测试: test_note_resolve_query_contains_name_single_match
假设 vault 中存在名称为 `绿联科技` 的 note，且查询为更长名称
当   执行 `markbase note resolve "深圳绿联科技有限公司"`
那么 JSON `status` 为 `query_contains_name`
并且 第一条 match 的 `matched_by` 为 `query_contains_name`

场景: alias 仍然优先于两类部分名称匹配
测试: test_note_resolve_alias_ranks_before_partial_name_matches
假设 note-a 的 alias 精确包含 `绿联`
并且 note-b 的名称为 `绿联科技`
当   执行 `markbase note resolve "绿联"`
那么 alias 命中的候选排在部分名称命中之前
并且 若最终只有一条 alias 候选，则 `status` 为 `alias`

场景: 同一笔记同时满足多种规则时，只返回一次且采用最高优先级来源
测试: test_note_resolve_deduplicates_match_sources_by_priority
假设 某 note 的 `name` 与 `alias` 都可能让同一查询命中
当   执行 `markbase note resolve`
那么 返回结果中该 note 只出现一次
并且 `matched_by` 为该 note 可获得的最高优先级匹配来源

场景: README 对 resolve 新状态和匹配来源的说明与实现一致
测试: test_note_resolve_behavior_matches_readme_contract
假设 README 已记录 `exact`、`alias`、`name_contains_query`、`query_contains_name`、`multiple`、`missing`
当   执行覆盖这些状态的 CLI 回归测试
那么 README 中的用户可见行为与实现一致

场景: name、alias 和部分名称匹配都不区分大小写
测试: test_note_resolve_partial_name_matching_is_case_insensitive
假设 vault 中存在名称为 `AcmePlatform` 的 note
并且 另一个 note 的 alias 为 `ACME Corp`
当   执行 `markbase note resolve "platform" "acme corp"`
那么 `platform` 的结果按 `name_contains_query` 命中 `AcmePlatform`
并且 `acme corp` 的结果按 `alias` 命中对应 note

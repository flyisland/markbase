---
id: task-0028
title: "增加 source 附件归档与校验命令"
status: completed
boundaries:
  allowed:
    - "src/main.rs"
    - "src/attachment.rs"
    - "src/extractor.rs"
    - "tests/cli_attachment.rs"
    - "tests/common.rs"
    - "README.md"
    - "ARCHITECTURE.md"
    - "docs/design-docs/draft/design-015-source-attachments.md"
    - "specs/active/task-0028-source-attachment-command.md"
  forbidden_patterns:
    - "src/query/**"
    - "src/renderer/**"
    - "src/web/**"
    - "src/db.rs"
    - "src/template.rs"
completion_criteria:
  - id: "cc-001"
    scenario: "将一个本地文件归档到 source 专属附件目录并写回证据记录"
    test: "test_source_attach_copies_file_writes_evidence_and_returns_json"
  - id: "cc-002"
    scenario: "重复归档内容相同的附件不会产生副本"
    test: "test_source_attach_is_idempotent_for_same_content"
  - id: "cc-003"
    scenario: "同名但内容不同的附件不会被覆盖"
    test: "test_source_attach_disambiguates_same_filename_with_different_content"
  - id: "cc-004"
    scenario: "附件校验能发现文件、哈希和源记录之间的不一致"
    test: "test_source_verify_attachments_detects_missing_and_tampered_files"
  - id: "cc-005"
    scenario: "非 source_input 笔记和不存在的输入文件在写入前被拒绝"
    test: "test_source_attach_rejects_non_source_and_missing_input"
---

## Intent

为 Capture 的 source_input 笔记提供受控的附件归档命令，替代 Agent 通过
Shell 手动复制文件、计算哈希、再手改 Markdown 的流程。

目标是让一次附件归档成为一个可验证、幂等且 agent-friendly 的显式写操作：
输入为 source note 和本地文件，输出为库内副本及其结构化元数据；源笔记中的
“证据附件”区成为用户可读的证据清单。

本任务不实现 OCR、图片理解、PDF/文本内容摘要、远程 URL 下载、压缩包展开或
跨 source 的附件去重仓库。它只处理用户已提供、当前机器可读取的本地文件。

## Proposed CLI Contract

新增顶层 `source` 命令组：

```bash
markbase source attach <source-note> <input-path> --description <text>
markbase source attachments <source-note>
markbase source verify-attachments <source-note>
```

### `source attach`

示例：

```bash
markbase source attach \
  2026-07-24-09-57-43_元戎启行专有云迁移与流量核查 \
  /Users/ichen/Downloads/10d.txt \
  --description "100 个代码仓近 10 天变更的 think pack 计算结果"
```

输入规则：

- `<source-note>` 是 path-free 的 Markdown note 名，复用现有 note-name validator。
- 命令必须解析并读取该 note；只有 frontmatter `type: source` 的 note 可以作为目标。
- `<input-path>` 可以是绝对或相对路径；必须解析为一个存在、可读的普通文件。目录、
  符号链接、FIFO、设备文件及不可读文件必须在任何写入发生前失败。
- `--description` 必填、非空，作为证据用途说明；命令不得尝试从二进制或文本内容
  自动生成描述。

归档规则：

- 将附件复制（不是移动）到 source note 所在目录的
  `attachments/<source-note-file-stem>/` 下。例如：
  `sources/2026-..._流量核查.md` 的附件目录为
  `sources/attachments/2026-..._流量核查/`。
- 默认保留原始文件名。若目标中已存在同名文件且 SHA-256 相同，则不复制、不重复写入
  证据条目，返回 `status: "existing"`。
- 若同名文件 SHA-256 不同，绝不能覆盖；在扩展名前追加稳定序号，例如
  `report_02.txt`、`report_03.txt`，直到找到可用名称。
- 写入采用安全的临时文件加原子 rename；失败时不得留下半写入的最终文件或修改 source
  note。
- 至少记录 SHA-256、字节数、由文件类型推断的 MIME type、归档时的原始路径和描述。
- 本任务不设“文件过大”硬阈值；实现必须流式复制和流式哈希，避免把整个附件读入内存。

源笔记写回规则：

- 只允许更新 source_input 模板中的 `## 证据附件` 区。
- 该区必须由一个稳定的 machine-managed 标记包围；本任务应先为 source_input 模板定义
  该标记和兼容的既有文件迁移策略。不得根据自然语言标题或 callout 文本的模糊匹配
  来定位写入位置。
- 每个附件使用一条 Markdown 列表项，包含库内相对链接、描述、原始路径、SHA-256、
  字节数与 MIME type。显示链接必须相对 source note 文件，使 Obsidian 可直接打开。
- 不得改写 `## 原始输入` 的任何内容。
- 若目标 source 缺少受管附件标记，命令必须失败且不复制文件；不要猜测插入位置。

默认 stdout 为单行 JSON，供 Agent 直接消费；诊断信息写 stderr。成功的最小形态：

```json
{"source":"sources/2026-..._流量核查.md","status":"copied","attachment":{"path":"sources/attachments/2026-..._流量核查/10d.txt","sha256":"...","bytes":123,"mime_type":"text/plain","description":"..."}}
```

`status` 只能是 `copied` 或 `existing`。失败时返回非零退出码，stdout 不得输出成功 JSON。

### `source attachments`

读取 source note 的受管附件记录并以 JSON 数组输出，顺序与源笔记一致。此命令只读，
不创建目录、不刷新索引、不修复异常。

### `source verify-attachments`

重新读取每项归档文件并流式计算 SHA-256，验证：

- source note 仍是 `type: source`；
- 每项记录都有可解析的受管元数据；
- 库内相对路径未逃逸 vault；
- 目标文件存在、是普通文件，且与记录的 SHA-256、字节数和 MIME type 一致；
- 不存在两个记录指向同一归档路径却声称不同内容的情况。

全部通过时以 JSON 返回 `{"ok":true,"attachments":[...]}`。发现任一问题时返回非零，
并输出结构化 issue 列表，包含 `path`、`code`、`message`；校验命令绝不修复或删除文件。

## Decisions

- 命令命名为 `source`，而非泛化的 `note attach`：本阶段解决的是 source-first Capture 的
  证据保全，并且 source_input 已有明确的“证据附件”语义。把任意笔记附件、媒体库或
  文件引用抽象一起设计会无谓扩大范围。
- 附件元数据的权威记录在 source Markdown 的受管区，不进入 DuckDB；数据库只继续作为
  可重建索引，符合 files-as-product 原则。
- `source attach` 是显式写路径。CLI 参数、目录创建、复制、哈希、Markdown 更新与输出
  路由应由 `src/main.rs` 编排；文件复制/哈希/受管区序列化放进新的 `src/attachment.rs`。
- 输入路径可以在 vault 外，因为用户提供的附件通常来自下载目录；但最终归档目标必须在
  `MARKBASE_BASE_DIR` 内，且基于已解析 source note 的父目录计算。
- 本任务不把附件作为 note 交由 `note resolve`，也不要求扫描器为每个附件创建独立实体。
  附件保留标准 Markdown 相对链接和文件系统路径即可。
- 为防止 CLI 新接口成为隐性上下文负担，attach/list/verify 的默认输出必须为稳定 JSON，
  且不打印文件内容、base64 或哈希计算过程。

## Boundaries

### Allowed Changes

- `src/main.rs`：Clap 参数、命令编排、stdout/stderr 与退出码。
- `src/attachment.rs`：纯粹的路径校验、流式复制/哈希、MIME 推断、受管附件区解析与写回。
- `src/extractor.rs`：仅当需要让 source/type 读取复用既有 frontmatter extractor 时调整；
  不得新增第二套 frontmatter parser。
- `tests/cli_attachment.rs`、`tests/common.rs`：repo-owned `TestVault` 端到端覆盖。
- `README.md`：命令概览、示例与行为限制。
- `ARCHITECTURE.md`：增加该显式写路径及其 source-of-truth 边界。
- `docs/design-docs/draft/design-015-source-attachments.md`：若实现前需要细化受管标记或
  恢复策略，先写设计文档；实现完成后按文档系统规则推进状态。

### Forbidden

- 不得在 `src/query/**`、`src/renderer/**`、`src/web/**`、`src/db.rs`、`src/template.rs`
  中实现附件专属逻辑。
- 不得把附件字节、哈希或唯一元数据只存入 DuckDB。
- 不得修改用户的原始输入区，或通过全文件字符串替换改写无关 Markdown。
- 不得在同名冲突时覆盖已有归档文件。
- 不得递归复制目录、跟随符号链接、自动下载 URL 或读取附件内容以推断描述。
- 不得依赖开发者电脑路径、个人 vault 或仓库外 fixture 作为自动化测试前提。

## Completion Criteria

场景: 将一个本地文件归档到 source 专属附件目录并写回证据记录  
测试: `test_source_attach_copies_file_writes_evidence_and_returns_json`  
假设 `TestVault` 创建了一个带有效受管附件区的 source_input note 和一个库外文本 fixture  
当执行 `markbase source attach <source> <fixture> --description <text>`  
那么附件出现在 `<source-parent>/attachments/<source-stem>/` 下  
并且其字节与源 fixture 完全一致  
并且 source note 的受管区含有相对链接、描述、原始路径、SHA-256、字节数与 MIME type  
并且 stdout 是可解析的单行 JSON，stderr 不包含成功结果  
并且 `markbase source verify-attachments <source>` 成功。

场景: 重复归档内容相同的附件不会产生副本  
测试: `test_source_attach_is_idempotent_for_same_content`  
假设同一 source 已归档 `report.txt`  
当第二次以内容相同但路径可不同的文件执行 attach  
那么不会生成第二个归档文件或重复证据条目  
并且 stdout 的 `status` 为 `existing`。

场景: 同名但内容不同的附件不会被覆盖  
测试: `test_source_attach_disambiguates_same_filename_with_different_content`  
假设同一 source 已归档内容为 A 的 `report.txt`  
当归档另一份内容为 B 的 `report.txt`  
那么原 `report.txt` 内容保持为 A  
并且新文件使用不冲突名称、拥有 B 的哈希并新增一条证据记录。

场景: 附件校验能发现文件、哈希和源记录之间的不一致  
测试: `test_source_verify_attachments_detects_missing_and_tampered_files`  
假设已成功归档至少一个附件  
当测试分别删除归档文件、篡改归档字节、或篡改受管元数据  
那么 `source verify-attachments` 每次均以非零退出  
并且 JSON issue 包含稳定错误代码和受影响路径  
并且命令不会自行恢复、覆盖或删除任何文件。

场景: 非 source_input 笔记和不存在的输入文件在写入前被拒绝  
测试: `test_source_attach_rejects_non_source_and_missing_input`  
假设 vault 中有普通 note、有效 source note 和不存在的输入路径  
当尝试向普通 note attach 或使用不存在的输入路径 attach  
那么命令以非零退出且返回可行动错误  
并且不会创建附件目录、归档副本或修改任何 source Markdown。

## Validation

实现 Agent 完成前必须运行：

```bash
RUSTC_WRAPPER= cargo test
RUSTC_WRAPPER= cargo clippy -- -D warnings
RUSTC_WRAPPER= cargo fmt --check
```

并手动验证一次 `source attach`、`source attachments` 与
`source verify-attachments` 的 JSON 输出可被 `jq` 解析。

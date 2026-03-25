# markbase

一款面向智能体、兼容 Obsidian 的结构化 Markdown 笔记工作流命令行工具。

English version: [README.md](README.md)

markbase 针对的是这样一种明确的工作流：笔记写成普通 Markdown，知识库保持与 Obsidian 兼容，通过 template 和 `note verify` 让 agent 写出的笔记保持稳定结构，并让 agent、CLI 和 web 都能直接操作这个 Markdown 知识库。

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/flyisland/markbase)

## Why This Exists

我开发 markbase，主要是为了解决四类反复出现的问题：

1. Obsidian 虽然提供了 CLI，但前提是桌面 App 必须先打开，这使它并不适合无头环境或服务端 agent 工作流。
2. 即使有 AI 帮助，持续保持笔记结构一致依然很难。只要没有清晰契约，agent 就很容易在 frontmatter 和正文结构上“自由发挥”。
3. 官方的 Obsidian Base，或者社区的 Dataview 插件，都非常适合在笔记内展示一对多关系。比如客户公司笔记里可以自动展示相关人员和商机活动，但 agent 直接读取 Markdown 文件时是看不到这些派生视图的。
4. 当知识库同步到 remote server，并由 AI agent 更新后，仍然需要一种简单的方法，让人能快速通过浏览器查看笔记。

markbase 就是为填补这些空白而开发的。

其中最关键的是它的 template 系统。template 定义一类笔记应有的结构，而 `markbase note verify` 则让 agent 和人类都能持续检查这些笔记是否仍然符合这些结构约束，避免内容随着时间慢慢漂移成互不兼容的格式。

## Best Practice

我在实践中发现，最可靠的方式不是“让 agent 随意写 Markdown”，而是“由人类提供意图，由 agent 执行重复性的笔记工作，由 markbase 提供这套框架”。

```mermaid
flowchart TD
    H["人类<br/>提供意图与判断"]
    A["Agent / Skill<br/>选择 template 并起草更新"]
    T["Templates<br/>文件名、目录、字段、可写章节"]
    N["创建 / 更新<br/>markbase note new"]
    V["校验<br/>markbase note verify"]
    R["查看结果<br/>note render、.base、web serve"]

    H --> A
    A --> T
    T --> N
    N --> V
    V --> R
    R --> H
```

### 协作分工

| 角色 | 主要职责 | 不应做什么 |
| --- | --- | --- |
| 人类 | 提供意图、处理歧义、审阅边界情况 | 手工维护每篇笔记的全部 schema 细节 |
| Agent | 选择 template、填写允许的章节、对齐实体、修复 verify 失败 | 擅自发明新结构，或写入 template 未声明的区域 |
| markbase | 暴露 template、在正确位置创建笔记、校验 schema、渲染 `.base` 关系、提供 web 访问 | 取代人类判断，或依赖未经约束的自由输出 |

### 推荐工作流

| 步骤 | 主导方 | 发生什么 |
| --- | --- | --- |
| 1. 捕获意图 | 人类 | 用户说明发生了什么，或者想记录什么 |
| 2. 选择一个 template | Agent + markbase | agent 通过 `markbase template list` 和 `markbase template describe` 精确选择一个 template |
| 3. 创建笔记 | markbase | `markbase note new` 以正确的目录、默认值和结构创建文件 |
| 4. 只填写允许的部分 | Agent | agent 只写 template 明确允许填写的字段和章节 |
| 5. 校验结构 | markbase + agent | `markbase note verify` 发现结构漂移；如有问题，agent 负责修复 |
| 6. 查看派生视图 | 人类 + agent | 通过 `.base` 视图、`note render` 和 `web serve` 把关系结果再次暴露给双方 |

例如，一个兼容 Obsidian 的 CRM 风格知识库，可以这样使用 template：

- `company_customer` 可以用于定义 `entities/company/` 下的公司档案，要求稳定字段如 `description` 和 `type`，约束 `owner -> person` 这样的链接，并嵌入与相关人员、活动记录相关的 Base 视图。
- `person_work` 可以用于定义 `entities/person/` 下的人物档案，要求关联公司，并把关系历史限制在 template 声明的章节内，而不是任由内容自由漂移。
- `activity_log` 可以用于定义 `logs/opportunities/` 下的事件型记录，要求 `date`、`activity_type`、`related_customer` 等字段，并以统一结构保存附件和参与人视图。
- `opportunity-capture`、`english-capture` 这类 domain skill，也可以把 template 视为文件名规则、必填属性、可写章节和写后校验的唯一事实来源。

在这个工作流里，template 不只是“生成初稿的脚手架”，而是一个可执行的契约。它告诉 agent：这是什么笔记、应该放在哪里、文件名怎么起、哪些链接合法、哪些章节可写，以及后续修改后哪些约束仍然必须成立。`note verify` 的作用，就是让这个契约真正变得可执行、可检查。

## What markbase does

- 作为独立 CLI 运行，不依赖 Obsidian App。
- 保持 Markdown 文件为事实源，并构建一个可重建的 DuckDB 索引用于快速读取。
- 兼容 Obsidian 习惯，包括 wikilink、embed、frontmatter 和 `.base` 文件。
- 通过 template 提供可重复的笔记结构，再通过 `note verify` 长期约束结构一致性。
- 为 agent 提供稳定命令，用于查询笔记、解析链接、基于 template 创建笔记以及检查 schema 合规性。
- 渲染嵌入笔记和 Base 视图，让 agent 可以读取人类在 Obsidian 中看到的那些派生关系。
- 通过 web 提供快速浏览能力，便于在本地或服务器上查看知识库。

## Use markbase if

- 你希望 agent 直接操作 Markdown 知识库，而不是一个专有数据库。
- 你需要 Obsidian 兼容性，尤其是链接、嵌入和 Base 风格的关系视图。
- 你需要比“自由 Markdown 加一点 prompt 运气”更强的结构约束，尤其是当多个人或多个 agent 都在写笔记时。
- 你希望在同步到服务器后，依然可以通过 CLI 和 web 方便访问知识库。

## Core ideas

- 文件才是产品本身，DuckDB 只是派生索引。
- Markdown 笔记的身份由名字决定，而不是路径。
- 当内部抽象和 Obsidian 兼容性冲突时，优先保证 Obsidian 行为。
- template 加 `note verify` 是保持 AI 写入结构一致性的核心框架。
- 默认输出优先面向 agent；给人看的表格是显式选择，而不是默认行为。

## Installation

从 crates.io 安装：

```bash
cargo install markbase
```

从源码构建：

```bash
git clone <repository-url>
cd markbase
cargo build --release
./target/release/markbase --help
```

需要 Rust 1.85+。DuckDB 已内置。

## Quick Start

先设置知识库目录：

```bash
export MARKBASE_BASE_DIR=/path/to/your/vault
```

查询笔记：

```bash
markbase query "author == 'Tom'"
markbase query "list_contains(file.tags, 'customer')"
markbase query "SELECT file.path, note.author FROM notes WHERE note.author = 'Tom'"
```

基于 template 创建一篇笔记：

```bash
markbase note new acme --template company
markbase note verify acme
```

查看原始 Markdown 中 agent 本来看不到的派生关系：

```bash
markbase note render acme
```

通过浏览器访问整个知识库：

```bash
markbase web serve --homepage /HOME.md
markbase web serve --cache-control "public, max-age=60"
```

不启动浏览器，直接检查最终 web 输出：

```bash
markbase web get /entities/person/alice.md  # 输出最终 web Markdown
markbase web get /entities/person/alice.md?fields=properties,links
```

## Command Overview

| Command | Purpose |
| --- | --- |
| `query` | 使用表达式语法或 SQL 查询已索引的知识库 |
| `note new` | 创建 Markdown 笔记，可选使用 template |
| `note verify` | 检查一篇笔记是否仍然符合其 template schema |
| `note rename` | 重命名笔记并重写 wikilink 与 embed |
| `note resolve` | 为 agent 链接场景解析实体名到现有笔记 |
| `note render` | 展开 note embeds 和 `.base` 视图，输出 agent 可读内容 |
| `template list` | 列出可用 template |
| `template describe` | 查看标准化后的 template 内容 |
| `web serve` | 以浏览器可访问的方式提供知识库 |
| `web get` | 输出某个 canonical route 对应的最终 Markdown |

## Concepts That Matter

### Query namespaces

- `file.*` 表示已索引的文件元数据，例如 `file.path`、`file.name`、`file.tags`、`file.mtime`
- `note.*` 表示 frontmatter 字段
- 裸字段，例如 `author`，是 `note.author` 的简写

示例：

```bash
markbase query "file.mtime > '2024-01-01'"
markbase query "author == 'Tom'"
markbase query "list_contains(file.tags, 'project')"
```

### Obsidian-compatible linking

Markdown note 的链接请使用名字，而不是路径：

```markdown
[[Acme]]
[[Zhang San]]
![[pipeline.base]]
```

frontmatter 里的链接值请保持为带引号的 Obsidian wikilink：

```yaml
company: "[[Acme]]"
owner: "[[Zhang San]]"
```

### Templates and verification

template 是 markbase 最核心的价值之一。它与 `note verify` 一起，为一类笔记创建定义了一套可重复的框架；而 `note verify` 会在后续被人类或 agent 多次修改后，继续检查它是否仍然符合 schema。

这意味着你可以放心让 agent 创建和更新 Markdown 笔记，而不必接受结构逐渐失控。你不需要每次都依赖 prompt 去“希望它写得一致”，而是先把结构定义在 template 里，再持续用 verify 检查整个知识库是否还遵守这个结构。

在 `log-notes` 这类实际工作流里，一个 template 往往同时定义：

- 目标目录和文件名规范
- 必填 frontmatter 和允许值
- 链接字段的目标类型约束，例如 `company -> company` 或 `owner -> person`
- 哪些章节允许 agent 写入
- 哪些 `.base` embeds 是结构的一部分，必须保留

这就是为什么 `note verify` 如此重要。它不仅仅在创建时检查结构，而是在后续长期修改中持续阻止结构漂移。

### Web delivery

`web serve` 提供一个浏览器友好的知识库视图。默认监听 `127.0.0.1:3000`。对外路由是基于路径的，但 markbase 内部仍然保持基于名字的 Obsidian 风格笔记身份模型。

对于 exported 和 dynamic 两种模式，都会满足以下行为：

- 请求 `/` 会返回 `index.html`
- 请求 `/index.html` 会返回同一个 docsify entry HTML
- docsify entry HTML 会让内部 `.md` 和 `.base` 文档链接继续在 docsify 内部导航
- 浏览器入口 HTML 会在前端升级 Obsidian-style callout，包括 foldable 的 `[!type]+` 和 `[!type]-`，同时保留多行正文结构
- docsify shell 会把现有左侧 docsify sidebar 作为统一的 `Outline` / `Properties` / `Links` 标签栏
- `Outline` 是默认标签，展示 docsify 自己的 sidebar 导航和标题 outline
- 在 canonical `.md` 笔记路由上，`Properties` 和 `Links` 会与 `Outline` 一起出现；在 `.base` 和其他不支持的路由上只保留 `Outline`
- 当前激活的 sidebar 标签面板拥有自己的滚动容器，因此较长的 `Properties` 内容会留在 sidebar 内，而不会把标签区挤到首屏以下
- sidebar 中的 note / base 链接会继续留在 docsify 内部，通过 `#/entities/company/acme.md` 这样的 hash 路由导航
- docsify TOC 的锚点跳转，例如 `#/note.md?id=heading`，会保持为页内跳转，而不会变成后端 metadata 请求
- `.base` 页面、shell 根路由，以及其他非 note 路由不会请求 metadata sidebar 数据
- 图片和附件等 binary resource URL 仍然会继续直接解析

默认情况下，`web serve` 会在所有响应上返回 `Cache-Control: no-store, no-cache, must-revalidate`，以及对应的旧式 no-cache headers。可以通过 `--cache-control <value>` 覆盖进程内所有响应的这个 header。

Web 路由对外基于已索引的 `file.path`，但内部渲染仍然按名字解析 Markdown 笔记和 `.base` 目标。canonical 的笔记或资源 URL 始终是 `/<file.path>`，并使用适合浏览器的 percent-encoding。

每个 `web serve` 请求都会在路由解析前刷新索引，并使用 request-scoped DuckDB handle。对于不带 query 参数的 Markdown 笔记和直接 `.base` 目标，server 返回的是可供 docsify/marked 渲染的 Markdown，而不是 HTML shell。对于 binary resource，则返回原始字节内容。

canonical Markdown note 路由还支持 metadata 模式，通过 `?fields=...` 指定：

- `?fields=properties`
- `?fields=links`
- `?fields=properties,links`

在 metadata 模式下，同一个 canonical `.md` 路由会返回 `application/json; charset=utf-8`，而不是 Markdown。响应始终包含 `file` 对象，并且只包含被请求的额外顶层字段。

metadata 模式当前只支持 canonical Markdown note 路由：

- `.md?fields=...` 返回 JSON metadata
- `.base?fields=...` 返回 `400 Bad Request`
- binary resource 路由带 `fields` 参数时返回 `400 Bad Request`
- 未知 query 参数、未知字段名，以及格式错误的 `fields` 语法都会返回 `400 Bad Request`

服务端 Markdown pipeline 会：

- 复用 note-render 语义来处理递归 `![[note]]` 展开、`.base` 展开、soft-failure placeholder 和 quote container 保留
- 将 `[[note]]` 链接改写为 canonical 的路径式 Markdown 链接
- 将非 Markdown 的 `![[...]]` resource embed 改写为标准 Markdown 图片或链接
- 从普通 Markdown 正文中移除 `%%comment%%`
- 保持 fenced code block 和 inline code span 的字面内容不变
- 在 v1 中保留 unresolved wikilink、unresolved resource embed、selector-based note embed 和 block-target note embed 的原样文本

`markbase web get <canonical-url>` 会输出和 `web serve` 对应同一路由时完全相同的 payload：

- 普通 `.md` 和 `.base` 路由输出 Markdown 正文
- `.md?fields=...` 输出 JSON metadata

如果 canonical URL 解析到的是 binary resource，`web get` 会以说明性错误退出，而不是直接输出字节流。

HTTP miss 和 bad-path 行为如下：

- route miss 返回 `404 Not Found`
- 非法的 percent-decoding 返回 `400 Bad Request`

`web init-docsify` 会写出单个 `index.html`，但正常浏览器使用并不依赖它。浏览器入口 HTML 会在前端升级 Obsidian-style callout，同时保持后端 web 合同仍然是 Markdown，并保留多行 callout 正文结构。

## Environment

- `MARKBASE_BASE_DIR`: 知识库目录。默认是当前目录。
- `MARKBASE_INDEX_LOG_LEVEL`: 自动索引时的输出级别。
- `MARKBASE_COMPUTE_BACKLINKS`: 是否在索引时计算 backlinks。

## Validation

本地开发建议运行：

```bash
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

## More docs

- [ARCHITECTURE.md](ARCHITECTURE.md): 系统结构图与核心不变量
- [AGENTS.md](AGENTS.md): 面向 coding agent 的仓库工作说明
- [docs/design-docs/implemented/design-010-query-subsystem.md](docs/design-docs/implemented/design-010-query-subsystem.md): query 行为
- [docs/design-docs/implemented/design-011-note-creation.md](docs/design-docs/implemented/design-011-note-creation.md): note creation 行为
- [docs/design-docs/implemented/design-002-render.md](docs/design-docs/implemented/design-002-render.md): render 行为
- [docs/design-docs/implemented/design-003-web-note-view.md](docs/design-docs/implemented/design-003-web-note-view.md): web 行为

## License

MIT

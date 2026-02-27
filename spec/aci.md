# 基于 ACI 原则的 mdb CLI 设计建议

ACI（Action-Centered Interaction）原则强调以"动作"为核心组织交互，让用户和 agent 都能快速、准确地表达意图并得到可驱动下一步行动的反馈。

---

## 一、命令动词的一致性

**现状问题：** `index`、`query` 是动词，`new`、`template` 偏向名词，语义不统一。

**建议：** 全部统一为动词形式，以用户意图为中心：

```bash
mdb index                          # 索引文件
mdb query "has(tags, 'todo')"      # 查询笔记
mdb create note my-note            # 创建笔记（替换 new）
mdb list templates                 # 列出模板
mdb list fields                    # 列出可用字段
mdb describe template daily        # 查看模板内容
```

`list` 专门服务于工具自身的元信息探索，`query` 专门服务于笔记内容检索，边界清晰互不重叠。

---

## 二、字段解析规则的简化

**现状问题：** 原设计兼容 Obsidian Base 的双命名空间写法（`file.*` vs `note.*`），规则隐晦，用户需要理解 shorthand 的解析优先级，认知负担重。由于该兼容性目前无实际用途，予以去除。

**新规则：** 字段解析统一为两步，无需命名空间前缀：

1. 先检查是否为**保留字段**（原生文件元数据）
2. 若不是，则在 **frontmatter 属性**中查找

保留字段列表：`path`, `folder`, `name`, `ext`, `size`, `ctime`, `mtime`, `content`, `tags`, `links`, `backlinks`, `embeds`

```bash
# 查询写法统一，无需区分命名空间
mdb query "has(tags, 'todo')"      # tags 是保留字段
mdb query "author == 'John'"       # author 是 frontmatter 属性
mdb query "name == 'readme'"       # name 是保留字段
mdb query "category == 'project'"  # category 是 frontmatter 属性
```

**字段冲突处理：** 当 frontmatter 中的字段名与保留字段冲突时，保留字段优先，并在索引时给出明确警告：

```
⚠ notes/my-note.md: frontmatter field 'name' conflicts with a reserved field and will be ignored.
```

**`list fields` 清晰标注两类字段的边界：**

```
$ mdb list fields
Reserved fields:  path, folder, name, ext, size, ctime, mtime,
                  content, tags, links, backlinks, embeds
Frontmatter:      author, category, status, date, title   (found across 142 notes)

Note: If a frontmatter field conflicts with a reserved field, the reserved field takes precedence.
```

---

## 三、反馈的及时性与结构化

**现状问题：** 关键执行信息依赖 `-v` 才可见，默认输出不足以让用户判断执行结果。

**建议：** 默认输出结构化摘要，无需 `-v` 即可掌握执行状态：

```
$ mdb index --base-dir ./notes
Indexing ./notes...
  ✓ 142 files indexed  (3 new, 5 updated, 0 errors)  [1.2s]
  ⚠ Skipped: notes/broken.md — invalid frontmatter (line 4)
```

`query` 结果附带计数：

```
$ mdb query "has(tags, 'todo')"
path                      mtime
────────────────────────  ───────────────────
./notes/task-a.md         2025-01-10 09:00:00
./notes/task-b.md         2025-01-12 14:30:00

2 results
```

---

## 四、错误的可恢复性

**现状问题：** 查询出错时的错误信息质量直接决定用户能否自行恢复，目前未作规范。

**建议：** 错误信息精确定位，并给出修复提示：

```
$ mdb query "has(tag, 'todo')"
Error: Unknown field 'tag'
  Did you mean: 'tags' (reserved field)?

  Tip: Run `mdb list fields` to see all available fields.
```

---

## 五、参数的正交性

**现状问题：** `--base-dir`、`--database` 等全局参数仅部分命令支持，用户在不同命令间需要记忆不同的覆盖方式。

**建议：** 全局参数在顶层统一声明，所有子命令均可继承覆盖：

```bash
mdb --base-dir ./other-notes query "has(tags, 'work')"
mdb --database /tmp/test.db index --base-dir ./notes
```

优先级保持不变：CLI 参数 > 环境变量 > 默认值。

---

## 六、Agent 工作流的支持

这是 mdb 区别于普通 CLI 工具的关键设计点。Agent 是程序化的，每一步都需要**结构化、可预测、无歧义**的输出，才能驱动下一步动作。

### Agent 工作流

```
list templates → describe template X → create note --template X → 完善 note
```

每一步的输出直接作为下一步的输入，整个链条自洽。

**`mdb list templates`：选择阶段**

```bash
$ mdb list templates -o json
[
  { "name": "daily", "path": "./templates/daily.md", "description": "日报模板" },
  { "name": "project", "path": "./templates/project.md", "description": "项目模板" }
]
```

**`mdb describe template X`：理解阶段**

直接输出模板文件的原始内容，模板本身即契约。Agent 从中理解字段语义和填写要求，因此**模板的编写质量至关重要**，建议在模板规范（MKS schema）中要求每个字段必须有清晰的 description 或注释。

```bash
$ mdb describe template daily
---
date: ""          # 日期，格式 YYYY-MM-DD，必填
mood: ""          # 今日心情，选填
summary: ""       # 今日总结，必填
tags: []
---

## 今日记录

（正文内容说明...）
```

**`mdb create note --template X`：创建阶段**

输出新文件路径及新文件内容，agent 拿到即可进入完善阶段，无需二次读取：

```bash
$ mdb create note my-note --template daily -o json
{
  "path": "./notes/my-note.md",
  "content": "---\ndate: \"\"\nmood: \"\"\n..."
}
```

**JSON 作为 agent 模式的一等公民**

建议通过环境变量在 agent 环境下统一固定输出格式，避免每条命令都带 `-o json`：

```bash
export MDB_OUTPUT=json
```

---

## 七、各阶段命令对照

| 用户/Agent 意图 | 命令 | 关键输出 |
|---|---|---|
| 索引文件 | `mdb index --base-dir ./notes` | 执行摘要（数量、耗时、错误） |
| 查询笔记 | `mdb query "has(tags, 'todo')"` | 结果列表 + 计数 |
| 探索可用字段 | `mdb list fields` | 保留字段 + frontmatter 字段 |
| 选择模板 | `mdb list templates` | 模板列表 + 描述 |
| 理解模板 | `mdb describe template X` | 模板原始内容 |
| 创建笔记 | `mdb create note --template X` | 新文件路径 + 文件内容 |

---

## 八、总结

| 维度 | 当前状态 | 建议方向 |
|---|---|---|
| 命令命名 | 动词/名词混用 | 统一以动词为中心 |
| 字段解析规则 | 双命名空间，规则隐晦 | 单一规则：保留字段优先，其次 frontmatter |
| 反馈质量 | 依赖 `-v` 才可见 | 默认输出结构化摘要 |
| 错误恢复 | 未规范 | 精确定位 + 修复提示 |
| 参数正交性 | 全局参数仅部分命令支持 | 顶层声明，全命令继承 |
| Agent 支持 | 无专门设计 | 输出自洽链条 + JSON 一等公民 |

mdb 的功能核心已经相当扎实，以上建议集中在**降低摩擦、提升透明度、支持 agent 工作流**三个方向，使工具对人和程序都同样友好。
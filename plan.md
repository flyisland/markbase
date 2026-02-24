# 设计方案：index 命令监控模式 (--watch)

## 1. 功能概述

为 `index` 命令增加 `--watch` 参数。启用后：
- 先执行一次完整的索引任务
- 然后持续监控 `base-dir` 目录
- 检测到 `.md` 文件变化时，自动执行增量索引
- 收到 `Ctrl+C` 信号时优雅退出

## 2. 架构设计

### 2.1 新增依赖

在 `Cargo.toml` 中添加文件监控库：

```toml
notify = "6.1"        # 文件系统监控
notify-debouncer-mini = "0.4"  # 防抖处理
```

### 2.2 CLI 参数变更

在 `src/main.rs` 的 `Commands::Index` 中添加：

```rust
#[derive(Subcommand)]
enum Commands {
    Index {
        #[arg(short, long)]
        force: bool,

        #[arg(short, long)]
        verbose: bool,

        // 新增参数
        #[arg(short, long, help = "Watch for file changes and re-index automatically")]
        watch: bool,
    },
    // ...
}
```

### 2.3 核心模块设计

新增 `src/watcher.rs`：

```rust
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

pub struct Watcher {
    debouncer: notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
}

impl Watcher {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let (tx, rx) = mpsc::channel();
        
        let mut debouncer = new_debouncer(Duration::from_secs(2), tx)?;
        debouncer.watcher().watch(path.as_ref(), RecursiveMode::Recursive)?;
        
        Ok(Self { debouncer })
    }

    /// 阻塞等待文件变化事件，返回变化的文件路径
    pub fn wait_for_changes(&self) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        // 使用 rx 接收变化事件
    }
}
```

### 2.4 增量索引支持

修改 `scanner.rs` 中的 `index_directory` 函数，增加可选的 `paths` 参数：

```rust
pub fn index_directory(
    dir: &Path,
    db: &Database,
    force: bool,
    verbose: bool,
    #[allow(dead_code)]
    paths: Option<Vec<PathBuf>>,  // 新增：仅索引指定文件
) -> Result<(), Box<dyn std::error::Error>> {
    // 如果提供了 paths，则只索引这些文件
    // 否则执行全量扫描（现有逻辑）
}
```

### 2.5 主循环设计

```rust
fn run_watch_mode(
    base_dir: &Path,
    db: &Database,
    force: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // 初始全量索引
    scanner::index_directory(base_dir, db, force, verbose, None)?;
    
    println!("Watching for changes in {} (Ctrl+C to stop)", base_dir.display());
    
    // 创建 watcher
    let mut watcher = watcher::Watcher::new(base_dir)?;
    
    loop {
        match watcher.wait_for_changes() {
            Ok(changes) => {
                if !changes.is_empty() {
                    println!("Detected {} file changes, re-indexing...", changes.len());
                    scanner::index_directory(base_dir, db, false, verbose, Some(changes))?;
                }
            }
            Err(e) => {
                eprintln!("Watch error: {}", e);
                break;
            }
        }
    }
    
    Ok(())
}
```

## 3. 实现步骤

### Step 1: 添加依赖
- 修改 `Cargo.toml`，添加 `notify` 和 `notify-debouncer-mini`

### Step 2: 修改 CLI
- 在 `src/main.rs` 的 `Commands::Index` 中添加 `--watch` 参数

### Step 3: 实现 watcher 模块
- 新建 `src/watcher.rs`
- 实现文件监控逻辑，使用 debouncer 避免频繁触发
- 区分处理 Create / Modify / Remove 事件

### Step 4: 修改 db 模块 - 新增删除方法
- 在 `src/db.rs` 中新增 `delete_document` 方法

### Step 5: 修改 scanner 模块
- 为 `index_directory` 添加可选的 `paths` 参数
- 支持增量索引（仅处理指定文件）
- 新增 `update_backlinks_for_changed` 函数支持增量 backlinks 更新

### Step 6: 修改 main.rs
- 在 `Commands::Index` 分支中处理 `--watch` 参数
- 实现监控循环，处理文件创建/修改/删除
- 实现 backlinks 增量更新逻辑

### Step 7: 添加测试
- 单元测试：watcher 模块
- 集成测试：完整监控流程

## 4. 行为细节

### 4.1 防抖策略
- 使用 2 秒的 debounce 窗口
- 避免连续快速变化导致频繁索引

### 4.2 变化类型检测与处理

监听以下事件并分别处理：

| 事件类型 | 处理逻辑 |
|----------|----------|
| **Create** | 检测到新 `.md` 文件后，调用 `scanner` 增量索引 |
| **Modify** | 检测到 `.md` 文件修改后，调用 `scanner` 增量索引（通过 mtime 比较判断是否真正变更） |
| **Remove** | 从数据库中删除该文件的记录 |

### 4.3 文件变更处理

**当前实现逻辑** (`scanner.rs:26-35`)：
```rust
// 通过 mtime 比较判断文件是否变更
if !force {
    if let Some(db_mtime) = db.get_mtime(&path_str)? {
        let file_mtime = fs::metadata(path)?.modified()?.duration_since(UNIX_EPOCH)?.as_secs() as i64;
        if file_mtime <= db_mtime {
            continue; // 跳过未变更的文件
        }
    }
}
```

### 4.4 文件删除处理

需要新增 `db.delete_document` 方法：

```rust
// src/db.rs
impl Database {
    pub fn delete_document(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.conn.execute(
            "DELETE FROM documents WHERE path = ?",
            params![path],
        )?;
        Ok(())
    }
}
```

在监控循环中处理删除事件：

```rust
match event.kind {
    EventKind::Create(_) | EventKind::Modify(_) => {
        // 增量索引（scanner 会通过 mtime 判断是否需要重新索引）
        scanner::index_directory(base_dir, db, false, verbose, Some(changes))?;
    }
    EventKind::Remove(_) => {
        // 从数据库删除记录
        let path_str = path.canonicalize()?.to_string_lossy().to_string();
        db.delete_document(&path_str)?;
        // 删除后需要重新计算 backlinks
        if let Some(name) = path.file_stem() {
            update_backlinks_after_delete(db, &name.to_string_lossy())?;
        }
    }
}
```

### 4.5 Backlinks 增量更新

**当前实现** (`scanner.rs:74-93`)：
```rust
// 每次 index_directory 都重新计算所有文件的 backlinks
let link_map = db.get_all_links()?;
let mut backlinks = std::collections::HashMap::new();

for (path, links) in &link_map {
    for link in links {
        let link_name = link.trim_end_matches(|c: char| c == '|' || c == '#').to_string();
        backlinks.entry(link_name).or_default().push(path.clone());
    }
}

for doc in &all_docs {
    if let Some(back_links) = backlinks.get(&doc.name) {
        let mut updated_doc = doc.clone();
        updated_doc.backlinks = back_links.clone();
        db.upsert_document(&updated_doc)?;
    }
}
```

**增量更新策略**：

1. **文件变更后**：重新计算变更文件的 backlinks
   - 获取该文件的所有 links
   - 对每个 link 目标，重新获取所有指向它的源文件
   - 更新目标文件的 backlinks

2. **文件删除后**：清理指向该文件的 backlinks
   - 找到所有曾经链接到该文件的文件
   - 更新这些文件的 backlinks（移除已删除的文件）

```rust
// src/scanner.rs - 新增函数

/// 增量更新 backlinks（用于文件变更后）
pub fn update_backlinks_for_file(
    db: &Database,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let link_map = db.get_all_links()?;
    
    let mut backlinks: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for (path, links) in &link_map {
        for link in links {
            let link_name = link.trim_end_matches(|c: char| c == '|' || c == '#').to_string();
            backlinks.entry(link_name).or_default().push(path.clone());
        }
    }
    
    // 更新变更文件的 backlinks
    if let Some(back_links) = backlinks.get(file_path) {
        let mut stmt = db.conn.prepare("SELECT * FROM documents WHERE path = ?")?;
        // 更新该文件的 backlinks
    }
    
    Ok(())
}

/// 删除文件后更新 backlinks
pub fn update_backlinks_after_delete(
    db: &Database,
    deleted_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // 找到所有链接到已删除文件的文档
    // 重新计算它们的 backlinks（移除指向已删除文件的链接）
    let link_map = db.get_all_links()?;
    
    let mut backlinks: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for (path, links) in &link_map {
        for link in links {
            let link_name = link.trim_end_matches(|c: char| c == '|' || c == '#').to_string();
            // 跳过已删除的文件
            if link_name != deleted_name {
                backlinks.entry(link_name).or_default().push(path.clone());
            }
        }
    }
    
    // 更新所有受影响文件的 backlinks
    for (path, links) in &link_map {
        if let Some(back_links) = backlinks.get(path) {
            // 更新该文件的 backlinks
        }
    }
    
    Ok(())
}
```

### 4.6 退出机制
- 捕获 `SIGINT` (Ctrl+C) 信号
- 优雅退出：打印退出信息，关闭 watcher

### 4.7 日志输出
- 初始索引完成后的提示信息
- 检测到变化时的日志（区分创建/修改/删除）
- 每次增量索引的文件数量

## 5. 使用示例

```bash
# 监控当前目录
mdb index --watch

# 监控指定目录
mdb index --watch -b /path/to/notes

# 监控并显示详细输出
mdb index --watch -b /path/to/notes -v
```

## 6. 风险与限制

1. **单写者限制**: DuckDB 只支持单写者，监控模式下如果用户同时运行另一个 index 命令会失败
2. **文件锁定**: 增量索引期间原文件不能被占用
3. **大目录**: 超大目录初始索引可能较慢
4. **性能**: 频繁文件变化可能导致持续 indexing，需合理设置 debounce

## 7. 测试计划

| 测试项 | 描述 |
|--------|------|
| `watcher_new` | 测试 watcher 初始化 |
| `watcher_detect_create` | 测试检测文件创建 |
| `watcher_detect_modify` | 测试检测文件修改 |
| `watcher_detect_delete` | 测试检测文件删除 |
| `scanner_incremental` | 测试增量索引 |
| `scanner_update_backlinks` | 测试增量更新 backlinks |
| `scanner_delete_backlinks` | 测试删除文件后更新 backlinks |
| `db_delete_document` | 测试从数据库删除文档 |
| `cli_watch_flag` | 测试 CLI 参数解析 |
| `integration_watch` | 集成测试：监控+索引循环 |
| `integration_watch_delete` | 集成测试：文件删除 + backlinks 更新 |

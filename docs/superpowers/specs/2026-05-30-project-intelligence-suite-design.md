# Projector 项目智能套件设计

> 设计日期：2026-05-30
> 状态：待实现

## 背景

Projector 已具备完整的项目扫描、快照、报告、趋势、导出等基础功能。当前版本可以回答"项目健康如何"，但无法回答"我的依赖中哪些是共享的""哪些项目已经死了""我最近在忙什么"这类跨项目问题。

本次扩展围绕 **B+C+D** 方向（跨项目智能 + 深度分析 + 发现与组织），引入三个新命令和一个标签系统。

## 设计原则

- **增量交付**: 每个命令独立可发布，互不阻塞
- **复用现有架构**: 依赖分析复用 `analyzer.rs` 的文件遍历逻辑，标签挂载到现有 `list`/`report`/`scan` 流程
- **零外部依赖**: 所有依赖文件解析用现有 `toml` 和 `serde_json` crate，不新增依赖
- **兼容性**: 不破坏已有命令和行为

## 范围总览

| 命令 | 说明 | 核心文件变更 |
|------|------|-------------|
| `projector deps` | 跨项目依赖分析 | 新增 `src/subcmd/deps.rs`; 新增 `src/analyzer.rs` 依赖解析函数 |
| `projector orphans` | 定位孤儿项目 | 新增 `src/subcmd/orphans.rs`; 复用快照数据 |
| `projector activity` | 近期活动摘要 | 新增 `src/subcmd/activity.rs`; 复用 `count_commits` |
| `projector tag` | 项目标签管理 | 新增 `src/subcmd/tag.rs`; 新增 `src/tags.rs`; 增强 `list.rs`/`report.rs` |
| `projector search` | 跨项目搜索 | 新增 `src/subcmd/search.rs` |

## 数据模型

### 标签索引 (`~/.projector/tags.toml`)

```toml
[tags]
frontend = ["/home/user/projects/web-app", "/home/user/projects/blog"]
archived = ["/home/user/projects/old-tool"]
rust = ["/home/user/projects/projector"]
```

Rust 结构：

```rust
struct TagsIndex {
    tags: HashMap<String, Vec<String>>,  // tag_name -> [project_paths]
}
```

### 依赖条目（计算产物，不持久化）

```rust
struct DependencyEntry {
    name: String,          // 依赖名，如 "serde"
    version_req: String,   // 版本约束，如 "1.0" 或 "^2.0"
    project_path: String,  // 所属项目路径
    dep_type: String,      // "rust" / "js" / "python" / "go"
    is_dev: bool,          // 是否为开发依赖
}
```

## 命令详设

### 1. `projector deps`

跨项目依赖聚合与分析。

**CLI**:

```
projector deps                           # 全量依赖报告
projector deps <path>                    # 特定项目的依赖
projector deps --shared                  # 仅显示跨项目共享的依赖
projector deps --project <name>          # 过滤指定项目
projector deps -f json                   # JSON 输出
```

**依赖文件解析范围**：

| 语言 | 文件 | 解析方式 | 提取字段 |
|------|------|---------|---------|
| Rust | Cargo.toml | `toml` (已有依赖) | `[dependencies]`, `[dev-dependencies]` 的 key + version |
| JS/TS | package.json | `serde_json` (已有依赖) | `dependencies`, `devDependencies` |
| Go | go.mod | 逐行解析 `require (...)` 块 | module path + version |
| Python | pyproject.toml | `toml` | `project.dependencies` / `[tool.poetry.dependencies]` |
| Python | requirements.txt | 逐行解析 | 包名 + 版本约束 |

**输出格式** (终端):

```
  Dependency Report — 24 projects
  ========================================
  Shared dependencies (used by 2+ projects):
    serde          1.0    Rust    used by: projector, my-lib, web-rs
    tokio          1.35   Rust    used by: projector, web-rs
    react          18.2   JS      used by: web-app, admin-panel
    axios          1.6    JS      used by: web-app, admin-panel, api-gateway

  Per-project:
    projector       Rust    12 deps (1 dev)
    web-app         JS      24 deps (8 dev)
    ...
```

**JSON 输出结构**：

```json
{
  "shared": [{"name": "serde", "version": "1.0", "type": "rust", "projects": ["proj1", "proj2"]}],
  "projects": [{"path": "projector", "total_deps": 12, "dev_deps": 1, "deps": [...]}],
  "total_projects": 24,
  "total_deps": 156,
  "unique_deps": 89,
  "shared_dep_count": 12
}
```

**特殊情况**：
- 无可扫描的项目（无快照） → 提示"请先执行 `projector scan`"
- 项目没有依赖文件 → 该项目的 deps 为空数组
- 依赖文件解析失败 → 跳过该项目，打印警告
- `--shared` 且没有共享依赖 → 显示"没有跨项目共享的依赖"
- 单项目模式 (`projector deps <path>`) 不依赖快照，直接分析

### 2. `projector orphans`

定位"孤儿项目"——既无远程追踪又长期无本地活动的项目。

**CLI**:

```
projector orphans                          # 默认阈值 90 天
projector orphans --days 180               # 自定义无活动天数
projector orphans -f json                  # JSON 输出
projector orphans --all                    # 显示所有项目状态（包括非孤儿）
```

**判定逻辑**：
- 有 `.git` 目录
- 无 `origin` 远程追踪（或从未推送过）
- 最新 commit 距今超过 `--days` 天（默认 90）
- 满足以上所有条件 → 标记为"孤儿"

**输出**：

```
  Orphan Projects (no remote + no activity >90d)
  ================================================
    old-experiment     Rust     last commit: 2025-01-15 (501d ago)
    playground         Python   last commit: 2025-03-20 (436d ago)
    scratchpad         JS       last commit: 2025-06-01 (363d ago)

  Total: 3 orphan projects out of 24 scanned
```

**特殊情况**：
- 无快照 → 提示"请先执行 `projector scan`"
- 无孤儿项目 → 显示"🎉 没有孤儿项目"
- 项目无 git → 不计入孤儿判断，在 `--all` 时标注"No git"

### 3. `projector activity`

跨项目近期活动摘要。

**CLI**:

```
projector activity                         # 默认最近 7 天
projector activity --days 30               # 最近 30 天
projector activity -f json                 # JSON 输出
projector activity --project <path>        # 单项目活动详情
```

**数据来源**：对每个项目实时调用 `git log` (复用 `analyzer::count_commits`)，不依赖快照。

**输出**：

```
  Activity — Last 7 days
  ========================================
  Total commits:       24
  Active projects:     5 / 24

  Hottest projects:
    projector          Rust     12 commits  (2 authors)
    web-app            JS        6 commits  (1 author)
    blog               Python    3 commits  (1 author)
    ...

  Idle projects (no activity):
    old-tool           Rust      last commit 2025-11-30 (181d ago)
    playground         Go        last commit 2026-01-15 (135d ago)
```

**特殊情况**：
- 某项目 git log 失败 → 跳过该项目，打印警告
- 无活动项目 → "最近 N 天没有检测到活动"
- `--project <path>` 显示该项目的提交历史摘要

### 4. `projector tag`

项目标签系统。

**CLI**:

```
projector tag list [<path>]                # 列出所有标签 / 某项目标签
projector tag set <path> <tag>             # 添加标签
projector tag rm <path> <tag>              # 移除标签
projector tag clear <path>                 # 清除所有标签
```

**集成**：

- `projector list --tag <tag>` — 仅显示带该标签的项目
- `projector report --filter tag=archived` — 报告过滤支持 `tag` 字段
- `projector scan` 时自动读取标签（不影响扫描逻辑，仅报告时使用）

**存储**：`~/.projector/tags.toml`，独立于 config 和 snapshots。

**特殊情况**：
- 标签名空或包含空格 → 报错
- 路径不存在或未扫描过 → 允许设置（路径不校验，标签只是元数据）
- `tag rm` 不存在的标签 → 静默成功
- 路径不存在于任何标签 → `tag list <path>` 返回空

### 5. `projector search`

跨项目搜索。轻量级文本搜索，不引入全文检索引擎。

**CLI**:

```
projector search <query>                   # 搜索项目名/类型/路径
projector search <query> --tag <tag>       # 在标签内搜索
projector search <query> -f json           # JSON 输出
```

**搜索范围**：基于最新快照，匹配以下字段：
- 项目名（路径最后一段）
- 项目完整路径
- 项目类型
- 标签名

**输出**：

```
  Search results for "web"
  ========================================
    web-app             JS           tag: frontend
    web-rs              Rust         tag: backend
    old-web-demo        JS           tag: archived

  3 results
```

**特殊情况**：
- 无快照 → "请先执行 `projector scan`"
- 无结果 → "没有匹配的项目"
- 搜索词为空 → 报错

## 文件变更

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `src/subcmd/deps.rs` | 新增 | 依赖分析子命令 |
| `src/subcmd/orphans.rs` | 新增 | 孤儿项目子命令 |
| `src/subcmd/activity.rs` | 新增 | 活动摘要子命令 |
| `src/subcmd/tag.rs` | 新增 | 标签管理子命令 |
| `src/subcmd/search.rs` | 新增 | 搜索子命令 |
| `src/tags.rs` | 新增 | 标签索引 IO 模块 |
| `src/analyzer.rs` | 增强 | 新增依赖解析函数 (`parse_dependencies`, `parse_cargo_deps`, `parse_package_json_deps`, `parse_go_mod_deps`, `parse_pyproject_deps`, `parse_requirements_txt`) |
| `src/command.rs` | 增强 | 新增 5 个子命令和参数 |
| `src/subcmd/mod.rs` | 修改 | 注册新模块 |
| `src/lib.rs` | 修改 | 注册 `tags` 模块 |
| `src/subcmd/list.rs` | 增强 | 支持 `--tag` 过滤 |
| `src/subcmd/report.rs` | 增强 | 支持 `tag` 过滤字段 |

## 配置文件变更

无。标签存储在独立文件 `~/.projector/tags.toml`，不修改 `config.toml`。

## 依赖变更

无。所有解析使用现有 `toml` 和 `serde_json` crate。

## 架构影响

- 标签系统 (`tags.rs`) 是独立模块，不依赖 snapshot 或 config
- 依赖解析函数挂在 `analyzer.rs` 中，与现有 `file_type_distribution` 等辅助函数同级
- `deps` 和 `activity` 可以脱离快照独立运行（直接读文件系统），`orphans` 和 `search` 需要快照
- `tag` 集成到 `list`/`report` 通过 `--tag` 参数，最小侵入

## 发布顺序

1. **Phase 1: 标签系统** — `tag` 子命令 + `tags.rs` + `list --tag` + `report --filter tag=`
2. **Phase 2: 依赖分析** — `deps` 子命令 + analyzer 依赖解析函数
3. **Phase 3: 搜索 + 活动** — `search` + `activity` 子命令
4. **Phase 4: 孤儿检测** — `orphans` 子命令

每个 Phase 可独立发布，建议按顺序开发（P1 的标签系统被 P2-P4 的搜索/过滤功能复用）。

# Projector 使用手册

> 个人项目统计工具 — 扫描 Git 项目、分析健康度、追踪趋势

版本: 0.0.1 | 配置路径: `~/.projector/config.toml` | 快照路径: `~/.projector/snapshots/`

---

## 安装

```bash
cargo install projector
```

## 命令概览

| 命令 | 功能 |
|------|------|
| [`list`](#list) | 列出指定目录下的项目 / 非项目目录 |
| [`scan`](#scan) | 扫描项目，保存 JSON 快照 |
| [`report`](#report) | 显示健康仪表盘，可对比差异、排序、筛选 |
| [`config`](#config) | 查看 / 修改配置 |
| [`activity`](#activity) | 查看项目提交活动统计 |
| [`deps`](#deps) | 分析项目依赖关系 |
| [`orphans`](#orphans) | 查找孤立项目（无远程 + 长期无活动） |
| [`search`](#search) | 搜索项目（名称、路径、类型、标签） |
| [`tag`](#tag) | 项目管理标签 |
| [`rank`](#rank) | 对项目排序（健康分、LOC、提交数等） |
| [`brief`](#brief) | 生成项目简报 |
| [`size`](#size) | 分析目录大小 |
| [`inspect`](#inspect) | 深度分析单个项目 |
| [`stats`](#stats) | 全局统计数据 |
| [`trend`](#trend) | 跨快照趋势图（ASCII） |
| [`snapshot`](#snapshot) | 快照管理（清理旧快照） |
| [`export`](#export) | 导出 HTML 仪表盘 |
| [`completion`](#completion) | 生成 shell 自动补全脚本 |

---

## list

列出目录下的 Git 项目目录和普通目录。

```bash
projector list [dir] [--tag <tag>]
```

- `dir` — 目标目录，默认 `.`
- `--tag <tag>` — 仅显示包含指定标签的项目

输出：每个项目显示项目名、检测到的语言类型、最后修改时间、标签（如有）。超过 30 天未修改的日期标红。

```bash
projector list                         # 列出当前目录项目
projector list ~/projects              # 列出指定目录
projector list --tag work              # 仅显示标签为 work 的项目
```

---

## scan

扫描目录中的 Git 项目，生成快照并保存到 `~/.projector/snapshots/`。

```bash
projector scan [dir]
```

- `dir` — 目标目录，默认使用配置 `scan.default_path`

扫描过程中：
- 跳过隐藏目录（`.` 开头）
- 识别每个项目的语言类型
- 分析 Git 健康度（分支、脏状态、未推送提交、最后提交时间）
- 估算代码行数（LOC）
- 计算健康分
- 保存 JSON 快照

如果项目健康分低于 `alert.health_threshold`（默认 40），会在扫描结束时显示警告及扣分原因。

---

## report

以最新快照为基础，显示健康仪表盘。

```bash
projector report [--diff] [-f json|md] [--sort <field>] [--filter <expr>...]
```

**参数：**

| 参数 | 说明 |
|------|------|
| `--diff` | 对比最近两次快照的差异 |
| `-f, --format` | 输出格式：`json` 或 `md`（默认终端表格） |
| `--sort <field>` | 排序字段，前缀 `-` 表示降序 |
| `--filter <expr>` | 筛选表达式，可多次使用 |

**排序字段：** `name` `type` `health` `loc` `lines_of_code` `branch` `last_commit`

```bash
projector report -f json              # JSON 输出
projector report -f md                # Markdown 表格输出
projector report --diff               # 显示与上次快照的差异
projector report --sort -health       # 按健康分降序
projector report --sort name          # 按名称升序
```

**筛选语法：** `<field>[:<op>]=<value>`

- 运算符：`eq`（默认）、`gte`、`lte`、`gt`、`lt`
- 筛选字段：`name` `type` `health` `loc` `dirty` `branch` `last_commit`

```bash
projector report --filter "type=Rust"                               # 仅 Rust 项目
projector report --filter "health:gte=80"                           # 健康分 ≥ 80
projector report --filter "dirty=true"                              # 仅脏状态项目
projector report --filter "type=Rust" --filter "health:gte=80"      # 多条件（AND）
```

---

## config

查看或修改配置。

```bash
projector config              # 查看当前配置
projector config set <key> <value>   # 修改配置
```

**配置项：**

| 键 | 类型 | 默认值 | 说明 |
|----|------|--------|------|
| `scan.default_path` | string | `.` | `scan` 的默认扫描目录 |
| `report.stale_threshold_days` | number | 90 | 超过此天数无提交视为 stale |
| `snapshot.keep_count` | number | 30 | `snapshot prune` 默认保留的快照数 |
| `alert.health_threshold` | number | 40 | 扫描时健康分低于此值发出警告 |

```bash
projector config set scan.default_path ~/projects
projector config set report.stale_threshold_days 60
```

配置文件位置：`~/.projector/config.toml`

---

## activity

查看项目的提交活动统计。基于最近一次快照中的项目列表，分析每个项目的提交数量。指定 `--project` 时直接分析该目录（无需快照）。

```bash
projector activity [--days <N>] [--project <path>] [-f json]
```

| 参数 | 说明 |
|------|------|
| `--days <N>` | 统计最近 N 天的提交（默认 7） |
| `--project <path>` | 仅分析指定项目路径（可选） |
| `-f, --format` | 输出格式：`json` |

```bash
projector activity                         # 最近 7 天所有项目活动
projector activity --days 7                # 最近一周
projector activity --project ~/projects/myapp   # 单个项目
projector activity -f json                 # JSON 格式
```

输出：
- 总提交数、活跃项目数 / 总项目数
- 最活跃项目列表（按提交数降序）
- 闲置项目列表（无提交活动）

---

## deps

分析项目的依赖关系。支持 Rust（Cargo.toml）、JavaScript（package.json）、Go（go.mod）、Python（pyproject.toml/requirements.txt）。

```bash
projector deps [path] [--shared] [--project <name>] [-f json]
```

| 参数 | 说明 |
|------|------|
| `path` | 项目路径（可选，默认基于最新快照扫描所有项目） |
| `--shared` | 仅显示被 2 个以上项目共享的依赖 |
| `--project <name>` | 按项目名称筛选（模糊匹配） |
| `-f, --format` | 输出格式：`json` |

```bash
projector deps                                            # 所有项目依赖一览
projector deps ~/projects/myapp                           # 单个项目依赖
projector deps --shared                                   # 共享依赖（跨项目复用）
projector deps --project myapp                            # 筛选名为 myapp 的项目
projector deps --shared -f json                           # JSON 格式共享依赖
```

`--shared` 输出：每个共享依赖显示名称、版本、类型、使用它的项目列表。

---

## orphans

查找孤立项目——未配置远程 origin 且超过指定天数无提交的项目。

```bash
projector orphans [--days <N>] [--all] [-f json]
```

| 参数 | 说明 |
|------|------|
| `--days <N>` | 无活动阈值天数（默认 90） |
| `--all` | 同时显示非孤立项目和无 git 的项目 |
| `-f, --format` | 输出格式：`json` |

```bash
projector orphans                         # 查找孤立项目（默认 90 天）
projector orphans --days 30               # 更严格：30 天无活动即视为孤立
projector orphans --all                   # 完整视图（含非孤立项目）
projector orphans -f json                 # JSON 格式
```

---

## rank

对项目排序。基于最新快照，按指定指标对全部项目排名。

```bash
projector rank [--by <metric>] [--reverse] [--type <filter>] [--top <N>] [--category] [-f json]
```

| 参数 | 说明 |
|------|------|
| `--by <metric>` | 排序指标：`health`（默认）、`loc`、`activity`（近 30 天提交数）、`age`（距最后提交天数）、`commits`（总提交数） |
| `--reverse` | 升序排列（默认降序） |
| `--type <filter>` | 按项目类型筛选（模糊匹配） |
| `--top <N>` | 仅显示前 N 名 |
| `--category` | 按类型分组，仅显示每种类型的第一名 |
| `-f, --format` | 输出格式：`json` |

```bash
projector rank                          # 按健康分降序排名
projector rank --by loc --top 5         # 代码行数最多的 5 个项目
projector rank --by activity            # 近 30 天最活跃的项目
projector rank --by age --reverse       # 最年轻的（最近有提交的项目）
projector rank --type Rust --top 3      # 仅 Rust 项目前 3 名
projector rank --category               # 每种语言的最佳项目
projector rank --by commits -f json     # JSON 格式
```

---

## brief

生成项目简报。基于最新快照，输出项目总数、健康分布、活动统计。

```bash
projector brief [--days <N>] [-f json]
```

| 参数 | 说明 |
|------|------|
| `--days <N>` | 统计最近 N 天的提交活动（默认 1） |
| `-f, --format` | 输出格式：`json` |

输出：
- 项目总数、平均健康分、健康分布（≥80 / 50–79 / <50）
- 总 LOC、脏状态项目数、stale 项目数
- 最近 N 天有提交的项目列表（按提交数降序）

```bash
projector brief                         # 最近 1 天的项目简报
projector brief --days 7               # 最近 7 天
projector brief --days 30 -f json       # JSON 格式，最近 30 天
```

---

## size

分析项目目录的磁盘占用。

```bash
projector size [path] [--top <N>] [--deep] [-f json]
```

| 参数 | 说明 |
|------|------|
| `path` | 指定项目路径（可选） |
| `--top <N>` | 仅显示最大的 N 个项目（不指定 `path` 时基于快照） |
| `--deep` | 指定 `path` 时按源码 / 依赖 / git 对象 / 其他分类统计 |
| `-f, --format` | 输出格式：`json` |

```bash
projector size                        # 基于快照显示所有项目的磁盘占用
projector size --top 5                # 最大的 5 个项目
projector size ~/projects/myapp       # 单个项目大小
projector size ~/projects/myapp --deep   # 深度分解（源码 / 依赖 / git / 其他）
projector size -f json                # JSON 格式
```

`--deep` 输出分类：
- **源码** — 常见代码扩展名的文件总大小
- **依赖** — `node_modules`、`target` 目录大小
- **Git** — `.git` 目录大小
- **其他** — 剩余部分

---
## search

搜索项目。基于最新快照，匹配项目名称、路径、类型和标签。

```bash
projector search <query> [--tag <tag>] [-f json]
```

| 参数 | 说明 |
|------|------|
| `query` | 搜索关键词（必填） |
| `--tag <tag>` | 仅搜索包含指定标签的项目 |
| `-f, --format` | 输出格式：`json` |

```bash
projector search rust                     # 搜索名称/路径/类型含 "rust" 的项目
projector search myapp                    # 搜索项目名含 "myapp" 的
projector search web --tag work           # 标签为 work 的项目中搜索 "web"
projector search rust -f json             # JSON 格式
```

---

## tag

管理项目的标签。标签存储在 `~/.projector/tags.toml`，用于分类和筛选项目。

```bash
projector tag list [path]          # 查看所有标签或指定项目的标签
projector tag set <path> <tag>     # 为项目添加标签
projector tag rm <path> <tag>      # 移除项目的指定标签
projector tag clear <path>         # 清除项目的所有标签
```

```bash
projector tag list                          # 列出所有标签及使用次数
projector tag list ~/projects/myapp         # 查看指定项目的标签
projector tag set ~/projects/myapp work     # 标记为 work
projector tag set ~/projects/myapp rust     # 标记为 rust
projector tag rm ~/projects/myapp work      # 移除 work 标签
projector tag clear ~/projects/myapp        # 清除项目所有标签
```

标签用于 `list --tag` 和 `search --tag` 筛选。

---

## inspect

深度分析单个项目目录。

```bash
projector inspect [path] [-f json]
```

- `path` — 项目路径，默认 `.`
- `-f, --format` — 仅支持 `json`

输出信息：
- 项目类型、代码行数、Git 分支、健康分
- 提交活动统计（总数 / 近 30 天 / 近 90 天 / 近 1 年 / 作者数）
- 文件类型分布（按语言分组 + 占比）
- 额外健康指标（stash 数量、未跟踪文件、落后上游提交数）
- 健康分扣减原因

```bash
projector inspect                        # 分析当前目录
projector inspect ~/projects/myapp       # 分析指定项目
projector inspect -f json                # JSON 格式输出
```

---

## stats

基于最新快照计算全局统计数据。

```bash
projector stats [-f json]
```

- `-f, --format` — `json`（默认终端表格）

输出：
- 项目总数、总 LOC、平均 / 中位 / 标准差健康分
- 健康分布：≥80（良好） / 50-79（一般） / <50（较差）
- 脏状态比例、stale 比例
- 语言类型分布
- Top 5 / Bottom 5 项目

---

## trend

跨多个快照绘制趋势图（ASCII）。

```bash
projector trend [path] [--days <N>] [--metric <name>] [-f json]
```

| 参数 | 说明 |
|------|------|
| `path` | 指定项目路径（可选，默认聚合所有项目） |
| `--days <N>` | 仅展示最近 N 天的快照 |
| `--metric` | `health`（默认）或 `loc` |
| `-f, --format` | `json` |

```bash
projector trend                        # 所有项目的平均健康分趋势
projector trend myapp                  # 单个项目健康分趋势
projector trend --metric loc           # 代码行数趋势
projector trend --days 90              # 最近 90 天
projector trend -f json                # JSON 格式
```

---

## snapshot

管理快照文件。

```bash
projector snapshot prune [--keep <N>] [--dry-run]
```

| 参数 | 说明 |
|------|------|
| `--keep <N>` | 保留最近 N 个快照，默认使用配置 `snapshot.keep_count`（30） |
| `--dry-run` | 模拟运行，不实际删除 |

```bash
projector snapshot prune               # 清理旧快照，保留 30 个
projector snapshot prune --keep 10     # 仅保留最近 10 个
projector snapshot prune --dry-run     # 预览会删除哪些文件
```

---

## export

导出 HTML 仪表盘。

```bash
projector export html [-o <output>]
```

| 参数 | 说明 |
|------|------|
| `-o, --output` | 输出 HTML 文件路径（默认输出到 stdout） |

```bash
projector export html                                   # 输出 HTML 到终端
projector export html -o dashboard.html                 # 导出到文件
```

---

## completion

生成 shell 自动补全脚本。

```bash
projector completion <bash|zsh|fish>
```

```bash
projector completion bash > /etc/bash_completion.d/projector
projector completion zsh > /usr/local/share/zsh/site-functions/_projector
projector completion fish > ~/.config/fish/completions/projector.fish
```

---

## 健康分公式

基础 **100 分**，按风险因素扣减：

| 扣减 | 条件 |
|------|------|
| -15 | `stale_threshold_days`（默认 90）无提交 |
| -10 | 工作区有未提交修改（dirty） |
| -5 × N | 未推送提交，每 5 个扣 5 分 |
| -10 | 文件最后修改在 60 天前 |
| -5 | 代码行数 < 100（可能是废弃脚手架） |

结果限制在 **0–100**。终端输出按颜色分类：
- **≥80** 绿色 — 健康
- **50–79** 黄色 — 一般
- **<50** 红色 — 较差

---

## 快照存储

- 快照文件：`~/.projector/snapshots/{YYYYMMDD_HHMMSS}.json`
- 每次 `scan` 生成一个新快照
- `report --diff` 对比最近两次快照
- `trend` 使用所有历史快照
- `snapshot prune` 清理过期快照

---

## 标签存储

- 标签文件：`~/.projector/tags.toml`
- TOML 格式，存储项目路径到标签列表的映射
- 用于 `list --tag` 和 `search --tag` 筛选
- 与快照独立管理，不会因 `snapshot prune` 丢失

---

## 配置参考

完整的 `~/.projector/config.toml`：

```toml
[scan]
default_path = "."

[report]
stale_threshold_days = 90

[snapshot]
keep_count = 30

[alert]
health_threshold = 40
```

## 推荐工作流

```bash
# 1. 设置扫描目录
projector config set scan.default_path ~/projects

# 2. 首次扫描
projector scan

# 3. 查看仪表盘
projector report

# 4. 为项目打标签分类
projector tag set ~/projects/work-project work
projector tag set ~/projects/hobby-project hobby

# 5. 按标签筛选查看
projector list --tag work

# 6. 深度分析问题项目
projector inspect ~/projects/some-project

# 7. 查看活动趋势
projector activity --days 30

# 8. 找出孤立项目
projector orphans

# 9. 查看跨项目共享依赖
projector deps --shared

# 10. 定期扫描（可加 cron）
projector scan && projector report --diff

# 11. 导出仪表盘分享
projector export html -o dashboard.html

# 12. 清理旧快照
projector snapshot prune --keep 20
```

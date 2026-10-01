# 🔦 Projector

> 个人项目统计工具

[![Crates.io](https://img.shields.io/crates/v/projector.svg)](https://crates.io/crates/projector)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

---

## 安装

```bash
cargo install projector
```

## 使用

```bash
# 列出目录下的项目
projector list [dir]

# 扫描项目并保存快照
projector scan [dir]

# 显示健康仪表盘
projector report

# 显示与上次快照的差异
projector report --diff

# 输出 JSON/Markdown 格式
projector report -f json
projector report -f md

# 查看当前配置
projector config

# 修改配置
projector config set scan.default_path ~/projects
projector config set report.stale_threshold_days 60
```

## 命令

完整使用手册见 [USAGE.md](USAGE.md)。

| 命令 | 说明 |
|------|------|
| `list [dir] [--tag]` | 列出目录下的项目 |
| `scan [dir]` | 扫描项目，保存快照到 `~/.projector/snapshots/` |
| `report [--diff] [-f json\|md]` | 显示健康仪表盘，支持排序、筛选、差异 |
| `activity [--days] [--project]` | 查看项目提交活动统计 |
| `brief [--days]` | 生成项目简报（总数、健康分布、活跃项目） |
| `deps [path] [--shared]` | 分析项目依赖关系（Cargo / npm / go / Python） |
| `orphans [--days] [--all]` | 查找孤立项目（无远程 + 长期无活动） |
| `rank [--by] [--top]` | 按健康分 / LOC / 活动度 / 项目年龄 / 提交数排序 |
| `search <query> [--tag]` | 搜索项目（名称、路径、类型、标签） |
| `size [path] [--top] [--deep]` | 分析目录大小 |
| `config [set <key> <value>]` | 查看 / 修改配置 |
| `inspect [path]` | 深度分析单个项目 |
| `stats` | 全局统计数据 |
| `trend [path] [--metric]` | 跨快照趋势图（ASCII） |
| `completion <shell>` | 生成 shell 自动补全脚本 |
| `export html [-o]` | 导出 HTML 仪表盘 |
| `snapshot prune [--keep] [--dry-run]` | 快照管理（清理旧快照） |
| `tag list\|set\|rm\|clear` | 项目管理标签 |

## 配置

`~/.projector/config.toml`

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

| 键 | 类型 | 默认值 | 说明 |
|----|------|--------|------|
| `scan.default_path` | string | `.` | `scan` 的默认扫描目录 |
| `report.stale_threshold_days` | number | 90 | 超过此天数无提交视为 stale |
| `snapshot.keep_count` | number | 30 | `snapshot prune` 默认保留的快照数 |
| `alert.health_threshold` | number | 40 | 扫描时健康分低于此值发出警告 |

## 健康分公式

基础 100 分，按风险因素扣减：

- **-15** — 超过 `stale_threshold_days`（默认 90 天）无提交 (stale)
- **-10** — 工作区有未提交修改 (dirty)
- **-5 × N** — 有未推送的提交（每 5 个提交扣 5 分）
- **-10** — 文件最后修改在 60 天前
- **-5** — 代码行数 < 100（可能是废弃脚手架）

结果限制在 0–100。终端输出按颜色分类：≥80 绿色、50–79 黄色、<50 红色。

## 存储路径

| 文件 | 路径 |
|------|------|
| 配置 | `~/.projector/config.toml` |
| 快照 | `~/.projector/snapshots/{YYYYMMDD_HHMMSS}.json` |
| 标签 | `~/.projector/tags.toml` |

## 开发

```bash
cargo build        # 开发构建
cargo test         # 运行测试（82 个单元测试）
cargo build --release
```

Rust edition 2024（MSRV ≥ 1.85）。依赖：`anyhow`、`chrono`、`clap`、`clap_complete`、`git2`、`serde`、`serde_json`、`toml`。

## 许可证

MIT © hunter-hongg

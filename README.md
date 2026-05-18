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
projector config set scan.max_depth 3
projector config set report.stale_threshold_days 60
```

## 命令

| 命令 | 说明 |
|------|------|
| `list [dir]` | 列出目录下的项目 |
| `scan [dir]` | 扫描项目，保存快照到 `~/.projector/snapshots/` |
| `report` | 根据最新快照显示健康仪表盘 |
| `report --diff` | 对比最近两次快照的差异 |
| `report -f json\|md` | 指定输出格式 (JSON/Markdown) |
| `config` | 查看当前配置 |
| `config set <k> <v>` | 修改配置项 |

## 配置

`~/.projector/config.toml`

```toml
[scan]
default_path = "."
max_depth = 1

[report]
stale_threshold_days = 90
```

## 健康分公式

基础 100 分，按风险因素扣减：

- **-15** — 90 天以上无提交 (stale)
- **-10** — 工作区有未提交修改 (dirty)
- **-5** — 有未推送的提交 (每 5 个提交)
- **-10** — 文件最后修改在 60 天前
- **-5** — 代码行数 < 100 (可能是废弃脚手架)

结果限制在 0–100。终端输出按颜色分类：≥80 绿色、50–79 黄色、<50 红色。

## 许可证

MIT © hunter-hongg

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
# 扫描当前目录
projector scan

# 递归扫描
projector scan ~/projects -r

# 生成报告
projector report
```

## 命令

| 命令 | 说明 |
| ------ | ------ |
| `scan [path]` | 扫描项目 |
| `report` | 生成报告 |
| `config` | 配置管理 |

## 选项

| 参数 | 说明 |
| ------ | ------ |
| `-r, --recursive` | 递归扫描 |
| `-f, --format` | 输出格式 (table/json) |
| `-v, --verbose` | 详细输出 |

## 配置

`~/.config/projector/config.toml`

```toml
[scan]
default_path = "~/projects"
max_depth = 3

[output]
default_format = "table"
```

## 许可证

MIT © hunter-hongg

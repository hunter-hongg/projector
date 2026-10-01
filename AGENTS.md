# Projector

> 个人项目统计工具 — Rust CLI

## 构建与运行

```bash
cargo build
cargo run -- list|scan|report|config
cargo build --release
```

Rust edition **2024** (MSRV ≥ 1.85)。不要假定 2021。

## 测试

测试以内嵌 `#[cfg(test)] mod tests` 形式存在于各源文件中（无独立测试目录）。新增代码必须添加测试：
```bash
cargo test
```

## 架构要点

- `src/main.rs` → clap derive 派发到 `src/subcmd/` 下的子命令
- `src/lib.rs` — 公开模块：analyzer / color / command / config / export_template / snapshot / subcmd / tags
- `src/command.rs` — 18 个子命令定义（list, scan, report, activity, brief, deps, orphans, rank, search, size, config, inspect, stats, trend, completion, export, snapshot, tag）
- `src/analyzer.rs` — 项目类型检测、git 健康、LOC 估算、健康分计算、依赖解析、ASCII 图表、统计聚合
- `src/snapshot.rs` — 快照序列化/加载/差异比较/修剪，JSON 存储
- `src/config.rs` — TOML 配置读写，路径 `~/.projector/config.toml`
- `src/tags.rs` — 项目标签管理，TOML 存储于 `~/.projector/tags.toml`
- `src/export_template.rs` — HTML 仪表盘模板生成（内嵌 Chart.js）
- `src/color.rs` — ANSI 终端颜色辅助

## 存储路径

- 配置: `~/.projector/config.toml`
- 快照: `~/.projector/snapshots/{YYYYMMDD_HHMMSS}.json`
- 标签: `~/.projector/tags.toml`

## 约定

- 中文文档、中文 README、中文提交信息
- 无 CI、无格式/林特配置 — 使用 `cargo fmt` 和 `cargo clippy`
- 无 TUI、无守护进程、无 Web 仪表盘（按设计；`export html` 生成静态文件除外）

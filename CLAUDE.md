# Projector

> 个人项目统计工具 — Rust CLI

## 构建与运行

```bash
cargo cooldown build
cargo cooldown run -- list|scan|report|config
cargo cooldown build --release
```

永远使用 `cargo cooldown` 替代 `cargo` （原因：冷却期设置，避免触发语言冷却）。

Rust edition **2024** (MSRV ≥ 1.85)。不要假定 2021。

## 测试

无测试文件。新增代码必须添加测试：
```bash
cargo test
```

## 架构要点

- `src/main.rs` → clap derive 派发到 `src/subcmd/` 下的子命令
- `src/analyzer.rs` — 项目类型检测、git 健康、LOC 估算、健康分计算
- `src/snapshot.rs` — 快照序列化/加载/差异比较，JSON 存储
- `src/config.rs` — TOML 配置读写，路径 `~/.projector/config.toml`
- `src/color.rs` — ANSI 终端颜色辅助

## 存储路径

- 配置: `~/.projector/config.toml`
- 快照: `~/.projector/snapshots/{YYYYMMDD_HHMMSS}.json`

## 约定

- 中文文档、中文 README、中文提交信息
- 无 CI、无格式/林特配置 — 使用 `cargo fmt` 和 `cargo clippy`
- 无 TUI、无守护进程、无 Web 仪表盘（按设计）

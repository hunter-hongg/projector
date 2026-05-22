# Projector 功能扩展设计

> 设计日期：2026-05-22
> 状态：待实现

## 背景

Projector 是一个个人项目统计 Rust CLI 工具。当前版本 (v0.0.1) 具备项目发现、扫描分析、快照存储、报告展示等基础功能。本次扩展围绕"A+C"方向（深度洞察 + 工作流集成），分四个 Phase 逐步增强工具能力。

## 设计原则

- **YAGNI**: 每个功能只做明确需要的事，不做过度工程
- **与现有架构一致**: 延续 analyzer → snapshot → subcmd 的分层结构
- **增量交付**: 每个 Phase 独立可发布
- **兼容性**: 不破坏已有命令和行为

## 范围总览

| Phase | 包含功能 | 核心文件变更 |
|-------|---------|-------------|
| P1 | `inspect`, `stats` | 新增 `src/subcmd/inspect.rs`, `src/subcmd/stats.rs`; 增强 `analyzer.rs` |
| P2 | `trend`, `report --sort/--filter` | 新增 `src/subcmd/trend.rs`; 增强 `report.rs`, `snapshot.rs` |
| P3 | `completion`, `export html` | 新增 `src/subcmd/completion.rs`, `src/subcmd/export.rs`; 新增依赖 `clap_complete` |
| P4 | `snapshot prune`, 健康分告警 | 新增 `src/subcmd/snapshot.rs`; 增强 `scan.rs`, `config.rs` |

---

## Phase 1: 深度洞察

### 1.1 `projector inspect <path>`

对单个已扫描或未扫描的项目做深度分析。

**CLI**:

```
projector inspect [path]             # 默认当前目录
projector inspect ~/projects/foo     # 指定路径
projector inspect -f json <path>     # JSON 输出
```

**数据维度**：

| 维度 | 实现方式 |
|------|---------|
| 基础信息 | 复用 `analyzer::analyze_project` 获取路径、类型、分支、健康分 |
| 版本活动 | 使用 `git2` 遍历 commit 历史，统计总数、最近 N 天频率、作者数 |
| 代码构成 | 新增文件类型分组统计（按扩展名聚合文件数和 LOC） |
| 仓库健康 | 检查是否有未跟踪文件、是否有 stash、branch 是否落后 upstream |

**特殊情况**：
- 路径不是 Git 仓库 → 降级输出（只有文件类型统计，无 git 指标），提示"非 Git 仓库，仅显示文件信息"
- 路径从未被 scan 过 → 即时分析，提示"即时分析（未找到快照）"
- 路径不存在 → 返回明确错误信息
- `-f json` → 输出结构化 JSON，便于脚本消费

**文件变更**：
- 新增 `src/subcmd/inspect.rs`
- 增强 `src/analyzer.rs`：
  - 新增 `count_commits()` — 统计不同时间窗口的 commit 数
  - 新增 `file_type_distribution()` — 文件类型分组统计
  - 新增 `git_extra_health()` — stash 数量、未跟踪文件、落后程度

**测试要求**：
- 非 Git 目录降级输出
- commit 频率统计边界（空仓库、单 commit）
- JSON 序列化正确性
- 文件类型分布正确性

---

### 1.2 `projector stats`

基于最新快照输出全局聚合统计。

**CLI**:

```
projector stats                   # 终端表格输出
projector stats -f json           # JSON 输出
```

**统计项**：

| 统计项 | 计算方式 |
|--------|---------|
| 项目总数 | `snapshot.projects.len()` |
| 类型分布 | 按 `project_type` 分组计数，计算占比 |
| 平均健康分 | `health_score` 的算术平均值 |
| 中位数健康分 | `health_score` 排序后的中位值 |
| 标准差 | 健康分总体标准差 |
| 健康分分布 | 三档计数：≥80 / 50-79 / <50 |
| Top 5 / Bottom 5 | 按健康分排序取首尾 |
| 总 LOC | `lines_of_code` 之和 |
| Dirty 比例 | `is_dirty=true` 项目占比 |
| Stale 比例 | 超过 `stale_threshold_days` 项目占比 |

**特殊情况**：
- 无快照 → 提示"未找到快照，请先执行 `projector scan`"
- 项目数量为 0 → 统计信息全为零，不报错
- 只有一个项目时，标准差为 0

**文件变更**：
- 新增 `src/subcmd/stats.rs`
- 新增 `src/analyzer.rs` 辅助函数 `compute_stats(snapshot)`

**测试要求**：
- 空快照统计
- 单一项目统计（标准差=0）
- 多项目统计数值正确性
- JSON / 终端格式输出

---

## Phase 2: 趋势分析

### 2.1 `projector trend`

跨多个快照展示健康分/LOC 随时间的变化趋势。

**CLI**:

```
projector trend                           # 所有项目平均健康分趋势
projector trend <path>                    # 单项目趋势
projector trend --days 90                 # 限制时间范围
projector trend --metric loc              # 切换为 LOC 趋势
projector trend -f json                   # JSON 输出
```

**实现细节**：
- 遍历 `~/.projector/snapshots/` 目录，按文件名排序加载所有快照
- 对于"所有项目"模式：计算每个快照中所有项目的平均健康分（或总 LOC）
- 对于"单项目"模式：在快照中匹配项目路径，提取该项目的健康分/LOC 序列
- ASCII 折线图：使用 `┌┐└┘├┤┬┴╭╮╰╯│─` 字符，自动缩放 Y 轴范围
- Y 轴标签左右对齐，X 轴显示快照日期（自动选择密度）

**约束**：
- 最低需要 2 个快照才能绘图（否则提示信息）
- 快照少于 3 个时不显示趋势线，只显示数据点
- 如果单项目在某个快照中不存在 → 该点跳过（不报错）
- `--days` 用于过滤快照的时间范围，与单项目可组合

**文件变更**：
- 新增 `src/subcmd/trend.rs`
- 新增 `src/snapshot.rs`：`load_all()` 方法，加载所有快照
- 新增 `src/analyzer.rs`：ASCII 图表绘制函数

**测试要求**：
- 0/1/2 个快照的边界行为
- 趋势序列正确提取
- 单项目快照缺失时的跳过逻辑
- JSON 输出的数值序列正确性

---

### 2.2 `report --sort` & `report --filter`

增强 `report` 命令的排序和过滤能力。

**CLI**:

```
projector report --sort health              # 升序
projector report --sort -health             # 降序
projector report --sort loc
projector report --sort name

projector report --filter type=Rust
projector report --filter health:gte=80
projector report --filter health:lte=50
projector report --filter dirty=true
projector report --filter type=Python --sort -health  # 组合
```

**排序字段**：`name`、`type`、`health`、`loc`、`branch`、`last_commit`

> 降序实现：`--sort` 的值解析器中检测 `-` 前缀（如 `-health`），将剩余字符串作为字段名并反转排序方向。clap 中 `-health` 不会被误解析为 flag，因为 `--sort` 已定义值类型，clap 会将后续 token 作为值处理。

**过滤语法**：`<field>:<op>=<value>`，冒号部分可省略（默认 eq）：
- `type=Rust` 等价于 `type:eq=Rust`
- `health:gte=80` — 支持 `eq`、`gte`、`lte`、`gt`、`lt`

**实现细节**：
- 排序：在 `latest.projects` 上用迭代器 `.sort_by()` 实现
- 过滤：在 `latest.projects` 上用迭代器 `.filter()` 实现
- 先过滤后排序
- 与 `-f json` / `-f md` 完全兼容
- 非法字段名 → 提示"无效排序/过滤字段"，列出可用字段
- 非法操作符 → 提示"无效操作符"，列出可用操作符

**文件变更**：
- 增强 `src/subcmd/report.rs`：解析 `--sort` 和 `--filter` 参数
- 新增 `src/analyzer.rs` 或 `src/snapshot.rs`：排序和过滤的辅助函数
- 更新 `src/command.rs`：增加新 CLI 参数

**测试要求**：
- 各排序字段正确性
- 各过滤操作符正确性（eq/gte/lte/gt/lt）
- 多条件 AND 过滤
- 排序+过滤组合
- 非法字段/操作符的错误信息

---

## Phase 3: 工作流集成

### 3.1 `projector completion <shell>`

生成 shell 补全脚本。

**CLI**:

```
projector completion bash       # 输出 bash 补全
projector completion zsh        # 输出 zsh 补全
projector completion fish       # 输出 fish 补全
```

**实现**：
- 新增依赖 `clap_complete`
- 利用 `clap::Command::complete()` 自动生成
- 所有补全逻辑由 clap_complete 接管，零手写

**约束**：
- 不支持的 shell → 提示"不支持的 shell：{shell}，支持 bash/zsh/fish"
- 输出到 stdout，不涉及文件操作

**文件变更**：
- 新增 `src/subcmd/completion.rs`
- 更新 `Cargo.toml`：新增 `clap_complete` 依赖
- 更新 `src/command.rs`：增加新子命令

**测试要求**：
- 各 shell 类型补全脚本生成（验证输出非空、含 `_projector` 函数名等特征）
- 非法 shell 类型的错误

---

### 3.2 `projector export html`

生成自包含 HTML 仪表盘。

**CLI**:

```
projector export html                        # 输出到 stdout
projector export html -o dashboard.html      # 写入文件
```

**HTML 内容**：
- 概览卡片：项目总数、平均健康分、总 LOC、dirty 比例、stale 比例
- 健康分布条形图：优秀/良好/危险三档，用纯 CSS 条形图（无 JS 图表库）
- 项目表格：项目名、类型、分支、状态、健康分，按健康分排序
- 类型分布：按项目类型计数的条形图
- Top 5 / Bottom 5 排行

**质量要求**：
- 单文件，所有 CSS/JS 内联
- 响应式设计，移动端友好
- 暗色/亮色自适应（`prefers-color-scheme`）
- 无外部资源依赖（无 CDN、无字体库）

**文件变更**：
- 新增 `src/subcmd/export.rs`
- 新增 `src/export_template.rs`（HTML 模板字符串，Rust 常量）
- 增强 `src/snapshot.rs`：提供 HTML 渲染所需的数据结构

**特殊情况**：
- 无快照 → 输出一个简化页面，提示"请先执行 scan"
- 无项目（快照中存在但 `projects` 空数组）→ 显示空状态页面
- `-o` 指定路径不存在父目录 → 自动创建

**测试要求**：
- HTML 输出包含关键 DOM 元素（#app、.stats-card、.project-table 等）
- 无快照时的空状态页面
- `-o` 文件写入正确性
- 各种健康分分布下的颜色正确性

---

## Phase 4: 运维提效

### 4.1 `projector snapshot prune`

清理旧快照，只保留最近的 N 个。

**CLI**:

```
projector snapshot prune --keep 10           # 保留 10 个
projector snapshot prune --keep 30           # 默认
projector snapshot prune --dry-run           # 预览模式
```

**配置扩展**：`config.toml` 新增：

```toml
[snapshot]
keep_count = 30
```

**实现细节**：
- 按文件名时间戳排序（YYYYMMDD_HHMMSS），保留 `--keep` 个最新
- 保留数 ≥ 当前快照总数时，不执行删除
- `--keep` 最小值 1，低于 1 报错
- 删除后打印被删除的文件名
- `--dry-run` 列出将删除的文件但不实际删除

**配置命令**：`projector config set snapshot.keep_count 50`

**文件变更**：
- 新增 `src/subcmd/snapshot.rs`（子命令）
- 增强 `src/config.rs`：新增 `snapshot` 配置段
- 增强 `src/snapshot.rs`：新增 `prune()` 方法

**测试要求**：
- 保留数 ≥ 总数时无操作
- `--keep 0` 报错
- dry-run 不实际删除
- 删除后剩余快照数正确
- 新配置项的读写

---

### 4.2 健康分阈值告警

在 `scan` 执行结束后自动检查并输出告警。

**配置扩展**：`config.toml` 新增：

```toml
[alert]
health_threshold = 40
```

**行为**：
- 仅在 `scan` 命令结束时触发，不影响命令退出码
- 低于阈值的项目按健康分升序列出
- 每项附带主要扣分原因（来自 `compute_health_score` 的各扣分项）
- 阈值设为 0 时关闭告警

**输出示例**：

```
Scanned 24 projects, snapshot saved.

⚠  以下项目健康分低于 40：
  - old-project     15/100  stale(412d), dirty
  - archived-lib    22/100  stale(2y), loc<100
```

**文件变更**：
- 增强 `src/subcmd/scan.rs`：扫描完成后检查并输出告警
- 增强 `src/config.rs`：新增 `alert` 配置段

**测试要求**：
- 阈值设为 0 时不输出告警
- 有项目低于阈值时正确输出
- 无项目低于阈值时无告警输出
- 新配置项读写

---

## 依赖变更

| 变更类型 | 包名 | 用途 | 影响 Phase |
|---------|------|------|-----------|
| 新增 | `clap_complete` | shell 补全自动生成 | P3 |
| 新增 | 无（复用现有 `git2`） | 深度 git 统计 | P1 |

无需其他外部依赖。`export html` 模板内联在 Rust 源码中（常量字符串）。

## 配置变更

```toml
[scan]
default_path = "."
max_depth = 1

[report]
stale_threshold_days = 90

[snapshot]          # ← 新增
keep_count = 30     # ← 新增

[alert]             # ← 新增
health_threshold = 40  # ← 新增
```

## 架构影响

- 无明显架构变更。所有新功能以新 `subcmd` 模块形式添加
- `analyzer.rs` 新增辅助函数，保持原有函数签名不变
- `snapshot.rs` 新增 `load_all()`、`prune()` 方法
- `config.rs` 新增配置段，与现有模式一致

## 发布顺序

1. **Phase 1** → `projector inspect` + `projector stats`
2. **Phase 2** → `projector trend` + `report --sort/--filter`
3. **Phase 3** → `projector completion` + `projector export html`
4. **Phase 4** → `projector snapshot prune` + 告警

每个 Phase 独立可发布，互不阻塞，但建议按顺序开发（P2 依赖 P1 的快照基础，P4 较为独立可提前）。

use serde::Serialize;

#[derive(Serialize)]
pub struct DashboardData {
    pub project_count: usize,
    pub avg_health: f64,
    pub total_loc: u32,
    pub dirty_ratio: f64,
    pub stale_ratio: f64,
    pub health_high: usize,
    pub health_mid: usize,
    pub health_low: usize,
    pub projects: Vec<DashboardProject>,
    pub type_distribution: Vec<TypeDistItem>,
    pub top5: Vec<RankItem>,
    pub bottom5: Vec<RankItem>,
    pub has_data: bool,
    pub scanned_at: String,
}

#[derive(Serialize)]
pub struct DashboardProject {
    pub name: String,
    pub project_type: String,
    pub branch: String,
    pub status: String,
    pub health: u8,
}

#[derive(Serialize)]
pub struct TypeDistItem {
    pub name: String,
    pub count: usize,
}

#[derive(Serialize)]
pub struct RankItem {
    pub name: String,
    pub health: u8,
}

pub fn render_html(data: &DashboardData) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Projector Dashboard</title>
<style>
  :root {{
    --bg: #ffffff;
    --text: #1a1a2e;
    --card: #f8f9fa;
    --border: #e0e0e0;
    --good: #2ecc71;
    --fair: #f39c12;
    --poor: #e74c3c;
  }}
  @media (prefers-color-scheme: dark) {{
    :root {{
      --bg: #1a1a2e;
      --text: #eaeaea;
      --card: #16213e;
      --border: #2a2a4a;
    }}
  }}
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: var(--bg); color: var(--text); padding: 20px; }}
  .container {{ max-width: 1200px; margin: 0 auto; }}
  h1 {{ font-size: 1.8rem; margin-bottom: 8px; }}
  .subtitle {{ color: #888; font-size: 0.9rem; margin-bottom: 24px; }}
  .stats {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr)); gap: 16px; margin-bottom: 32px; }}
  .stat-card {{ background: var(--card); border: 1px solid var(--border); border-radius: 12px; padding: 20px; text-align: center; }}
  .stat-card .value {{ font-size: 2rem; font-weight: 700; }}
  .stat-card .label {{ font-size: 0.8rem; color: #888; margin-top: 4px; }}
  .health-bar {{ display: flex; height: 32px; border-radius: 8px; overflow: hidden; margin-bottom: 32px; }}
  .health-bar .segment {{ display: flex; align-items: center; justify-content: center; font-size: 0.8rem; font-weight: 600; color: #fff; transition: width 0.3s; }}
  .bar-good {{ background: var(--good); }}
  .bar-fair {{ background: var(--fair); }}
  .bar-poor {{ background: var(--poor); }}
  .charts {{ display: grid; grid-template-columns: 1fr 1fr; gap: 16px; margin-bottom: 32px; }}
  @media (max-width: 640px) {{ .charts {{ grid-template-columns: 1fr; }} }}
  .chart-card {{ background: var(--card); border: 1px solid var(--border); border-radius: 12px; padding: 20px; }}
  .chart-card h3 {{ margin-bottom: 12px; font-size: 1rem; }}
  .bar-row {{ display: flex; align-items: center; margin-bottom: 6px; }}
  .bar-label {{ width: 100px; font-size: 0.85rem; flex-shrink: 0; }}
  .bar-track {{ flex: 1; height: 20px; background: var(--border); border-radius: 4px; overflow: hidden; }}
  .bar-fill {{ height: 100%; border-radius: 4px; transition: width 0.3s; }}
  .bar-count {{ width: 40px; text-align: right; font-size: 0.85rem; flex-shrink: 0; margin-left: 8px; }}
  .ranks {{ display: grid; grid-template-columns: 1fr 1fr; gap: 16px; margin-bottom: 32px; }}
  @media (max-width: 640px) {{ .ranks {{ grid-template-columns: 1fr; }} }}
  .rank-card {{ background: var(--card); border: 1px solid var(--border); border-radius: 12px; padding: 20px; }}
  .rank-card h3 {{ margin-bottom: 12px; font-size: 1rem; }}
  .rank-item {{ display: flex; justify-content: space-between; padding: 4px 0; font-size: 0.9rem; }}
  .table-wrap {{ overflow-x: auto; }}
  table {{ width: 100%; border-collapse: collapse; font-size: 0.85rem; }}
  th, td {{ padding: 8px 12px; text-align: left; border-bottom: 1px solid var(--border); }}
  th {{ font-weight: 600; }}
  .badge {{ display: inline-block; padding: 2px 8px; border-radius: 4px; font-size: 0.75rem; font-weight: 600; color: #fff; }}
  .badge-good {{ background: var(--good); }}
  .badge-fair {{ background: var(--fair); }}
  .badge-poor {{ background: var(--poor); }}
  .empty {{ text-align: center; padding: 60px 20px; color: #888; }}
  .empty h2 {{ margin-bottom: 8px; }}
</style>
</head>
<body>
<div class="container">
  <h1>📊 Projector Dashboard</h1>
  <p class="subtitle">Scanned at {scanned_at} · {project_count} projects</p>

  <div class="stats">
    <div class="stat-card">
      <div class="value">{project_count}</div>
      <div class="label">Projects</div>
    </div>
    <div class="stat-card">
      <div class="value">{avg_health:.1}</div>
      <div class="label">Avg Health</div>
    </div>
    <div class="stat-card">
      <div class="value">{total_loc}</div>
      <div class="label">Total LOC</div>
    </div>
    <div class="stat-card">
      <div class="value">{dirty_ratio:.0}%</div>
      <div class="label">Dirty</div>
    </div>
    <div class="stat-card">
      <div class="value">{stale_ratio:.0}%</div>
      <div class="label">Stale</div>
    </div>
  </div>

  <div class="health-bar">
    <div class="segment bar-good" style="width:{health_high_pct}%">{health_high}</div>
    <div class="segment bar-fair" style="width:{health_mid_pct}%">{health_mid}</div>
    <div class="segment bar-poor" style="width:{health_low_pct}%">{health_low}</div>
  </div>

  <div class="charts">
    <div class="chart-card">
      <h3>Health Distribution</h3>
      <div class="bar-row">
        <span class="bar-label">≥ 80 (Good)</span>
        <div class="bar-track"><div class="bar-fill" style="width:{health_high_pct}%;background:var(--good)"></div></div>
        <span class="bar-count">{health_high}</span>
      </div>
      <div class="bar-row">
        <span class="bar-label">50-79 (Fair)</span>
        <div class="bar-track"><div class="bar-fill" style="width:{health_mid_pct}%;background:var(--fair)"></div></div>
        <span class="bar-count">{health_mid}</span>
      </div>
      <div class="bar-row">
        <span class="bar-label">&lt; 50 (Poor)</span>
        <div class="bar-track"><div class="bar-fill" style="width:{health_low_pct}%;background:var(--poor)"></div></div>
        <span class="bar-count">{health_low}</span>
      </div>
    </div>
    <div class="chart-card">
      <h3>Type Distribution</h3>
      {type_chart}
    </div>
  </div>

  <div class="ranks">
    <div class="rank-card">
      <h3>🏆 Top 5</h3>
      {top5_html}
    </div>
    <div class="rank-card">
      <h3>⚠️ Bottom 5</h3>
      {bottom5_html}
    </div>
  </div>

  <div class="table-wrap">
    <table>
      <thead>
        <tr>
          <th>Project</th>
          <th>Type</th>
          <th>Branch</th>
          <th>Status</th>
          <th>Health</th>
        </tr>
      </thead>
      <tbody>
        {table_rows}
      </tbody>
    </table>
  </div>
</div>
</body>
</html>"#,
        scanned_at = data.scanned_at,
        project_count = data.project_count,
        avg_health = data.avg_health,
        total_loc = data.total_loc,
        dirty_ratio = data.dirty_ratio * 100.0,
        stale_ratio = data.stale_ratio * 100.0,
        health_high = data.health_high,
        health_mid = data.health_mid,
        health_low = data.health_low,
        health_high_pct = if data.project_count > 0 {
            (data.health_high as f64 / data.project_count as f64 * 100.0) as u32
        } else {
            0
        },
        health_mid_pct = if data.project_count > 0 {
            (data.health_mid as f64 / data.project_count as f64 * 100.0) as u32
        } else {
            0
        },
        health_low_pct = if data.project_count > 0 {
            (data.health_low as f64 / data.project_count as f64 * 100.0) as u32
        } else {
            0
        },
        type_chart = render_type_chart(&data.type_distribution, data.project_count),
        top5_html = render_rank_items(&data.top5, true),
        bottom5_html = render_rank_items(&data.bottom5, false),
        table_rows = render_table_rows(&data.projects),
    )
}

pub fn render_empty_html() -> String {
    r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Projector Dashboard</title>
<style>
  :root { --bg: #1a1a2e; --text: #eaeaea; --muted: #888; }
  @media (prefers-color-scheme: light) { :root { --bg: #ffffff; --text: #1a1a2e; --muted: #888; } }
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: var(--bg); color: var(--text); display: flex; align-items: center; justify-content: center; min-height: 100vh; }
  .empty { text-align: center; }
  .empty h1 { font-size: 2rem; margin-bottom: 12px; }
  .empty p { color: var(--muted); }
</style>
</head>
<body>
<div class="empty">
  <h1>📊 Projector Dashboard</h1>
  <p>No snapshot found. Run <code>projector scan</code> first.</p>
</div>
</body>
</html>"#.to_string()
}

fn render_type_chart(items: &[TypeDistItem], total: usize) -> String {
    let mut html = String::new();
    for item in items {
        let pct = if total > 0 {
            (item.count as f64 / total as f64 * 100.0) as u32
        } else {
            0
        };
        html.push_str(&format!(
            r#"<div class="bar-row"><span class="bar-label">{}</span><div class="bar-track"><div class="bar-fill" style="width:{}%;background:#3498db"></div></div><span class="bar-count">{}</span></div>"#,
            html_escape(&item.name),
            pct,
            item.count,
        ));
    }
    html
}

fn render_rank_items(items: &[RankItem], _is_top: bool) -> String {
    let mut html = String::new();
    for item in items {
        let badge_class = if item.health >= 80 {
            "badge-good"
        } else if item.health >= 50 {
            "badge-fair"
        } else {
            "badge-poor"
        };
        html.push_str(&format!(
            r#"<div class="rank-item"><span>{}</span><span class="badge {}">{}/100</span></div>"#,
            html_escape(&item.name),
            badge_class,
            item.health,
        ));
    }
    html
}

fn render_table_rows(projects: &[DashboardProject]) -> String {
    let mut rows = String::new();
    for p in projects {
        let badge_class = if p.health >= 80 {
            "badge-good"
        } else if p.health >= 50 {
            "badge-fair"
        } else {
            "badge-poor"
        };
        let status_class = match p.status.as_str() {
            "clean" => "badge-good",
            "dirty" => "badge-fair",
            _ => "badge-poor",
        };
        rows.push_str(&format!(
            r#"<tr><td>{}</td><td>{}</td><td>{}</td><td><span class="badge {}">{}</span></td><td><span class="badge {}">{}/100</span></td></tr>"#,
            html_escape(&p.name),
            html_escape(&p.project_type),
            html_escape(&p.branch),
            status_class,
            html_escape(&p.status),
            badge_class,
            p.health,
        ));
    }
    rows
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

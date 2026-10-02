// no external imports

pub struct TrendPoint {
    pub date: String,
    pub value: f64,
}

pub fn draw_ascii_chart(points: &[TrendPoint], width: usize, height: usize) -> Vec<String> {
    if points.is_empty() {
        return vec!["(no data)".to_string()];
    }

    let min_val = points.iter().map(|p| p.value).fold(f64::INFINITY, f64::min);
    let max_val = points
        .iter()
        .map(|p| p.value)
        .fold(f64::NEG_INFINITY, f64::max);

    if (max_val - min_val).abs() < f64::EPSILON {
        return vec![format!("all values = {:.1}", min_val)];
    }

    let plot_width = width.saturating_sub(8).max(2);
    let plot_height = height.saturating_sub(2).max(2);

    let mut lines = Vec::new();

    for row in 0..plot_height {
        let ratio = 1.0 - (row as f64 / (plot_height - 1) as f64);
        let val = min_val + ratio * (max_val - min_val);

        let show_label = val == min_val
            || val == max_val
            || (val - (min_val + (max_val - min_val) * 0.25)).abs() < (max_val - min_val) * 0.02
            || (val - (min_val + (max_val - min_val) * 0.5)).abs() < (max_val - min_val) * 0.02
            || (val - (min_val + (max_val - min_val) * 0.75)).abs() < (max_val - min_val) * 0.02;

        let label = if show_label {
            format!("{:.0}", val)
        } else {
            String::new()
        };

        let padded_label = if row % 2 == 0 || !label.is_empty() {
            format!("{:>6} ", label)
        } else {
            "       ".to_string()
        };

        let mut row_chars = String::with_capacity(plot_width);
        for col in 0..plot_width {
            let point_idx =
                (col as f64 / (plot_width - 1) as f64 * (points.len() - 1) as f64).round() as usize;
            let point_val = points[point_idx].value;
            let point_ratio = (point_val - min_val) / (max_val - min_val);

            let y_pos = (plot_height - 1) as f64 * (1.0 - point_ratio);
            let distance = (row as f64 - y_pos).abs();

            if distance < 0.5 {
                row_chars.push('●');
            } else if col > 0 {
                let prev_idx = ((col - 1) as f64 / (plot_width - 1) as f64
                    * (points.len() - 1) as f64)
                    .round() as usize;
                let prev_y = (plot_height - 1) as f64
                    * (1.0 - (points[prev_idx].value - min_val) / (max_val - min_val));
                let row_f = row as f64;
                if (row_f > prev_y && row_f < y_pos) || (row_f < prev_y && row_f > y_pos) {
                    row_chars.push('│');
                } else {
                    row_chars.push(' ');
                }
            } else {
                row_chars.push(' ');
            }
        }

        lines.push(format!("{}{}", padded_label, row_chars));
    }

    let x_axis = format!("       {}", "─".repeat(plot_width));
    lines.push(x_axis);

    let x_labels = self::format_x_axis_labels(points, plot_width);
    lines.push(format!("       {}", x_labels));

    lines
}

fn format_x_axis_labels(points: &[TrendPoint], width: usize) -> String {
    if points.is_empty() {
        return String::new();
    }
    let mut labels = vec![" ".to_string(); width];
    if let Some(first) = points.first() {
        let d = &first.date;
        for (i, c) in d.chars().enumerate() {
            if i < width {
                labels[i] = c.to_string();
            }
        }
    }
    if let Some(last) = points.last() {
        let d = &last.date;
        let start = width.saturating_sub(d.len());
        for (i, c) in d.chars().enumerate() {
            let pos = start + i;
            if pos < width {
                labels[pos] = c.to_string();
            }
        }
    }
    labels.concat()
}

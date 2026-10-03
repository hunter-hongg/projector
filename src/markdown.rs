//! Minimal GitHub-flavoured Markdown table building.
//!
//! Three commands emit Markdown (`report`, `brief`, `export md`). This owns the
//! one thing that is easy to get wrong by hand: a cell containing `|` or a
//! newline silently breaks the table, so every cell is escaped here.

/// Escape one cell: `|` would end the column, a newline would end the row.
pub fn escape_cell(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '|' => out.push_str("\\|"),
            '\n' => out.push(' '),
            '\r' => {}
            other => out.push(other),
        }
    }
    out
}

/// Render `headers` and `rows` as a Markdown table.
///
/// Emits nothing but the header when `rows` is empty, so callers keep control
/// of their own "no data" wording.
pub fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    out.push_str("| ");
    out.push_str(
        &headers
            .iter()
            .map(|h| escape_cell(h))
            .collect::<Vec<_>>()
            .join(" | "),
    );
    out.push_str(" |\n|");
    for _ in headers {
        out.push_str("---|");
    }
    for row in rows {
        out.push('\n');
        out.push('|');
        for cell in row {
            out.push(' ');
            out.push_str(&escape_cell(cell));
            out.push_str(" |");
        }
    }
    out
}

/// Render `value` as inline code, doubling backticks when it contains one.
///
/// Used for paths and branch names, where a stray underscore would otherwise
/// turn into emphasis.
pub fn inline_code(value: &str) -> String {
    let value = value.replace('\n', " ");
    if value.contains('`') {
        format!("`` {} ``", value)
    } else {
        format!("`{value}`")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_header_separator_and_rows() {
        let md = table(
            &["Name", "Health"],
            &[
                vec!["app-a".into(), "90".into()],
                vec!["app-b".into(), "42".into()],
            ],
        );
        let lines: Vec<&str> = md.lines().collect();
        assert_eq!(lines[0], "| Name | Health |");
        assert_eq!(lines[1], "|---|---|");
        assert_eq!(lines[2], "| app-a | 90 |");
        assert_eq!(lines[3], "| app-b | 42 |");
    }

    #[test]
    fn header_only_when_no_rows() {
        let md = table(&["Name"], &[]);
        assert_eq!(md, "| Name |\n|---|");
    }

    #[test]
    fn pipe_in_a_cell_cannot_break_the_table() {
        // A branch called `feat|x` used to be emitted raw, splitting the row.
        let md = table(&["Branch"], &[vec!["feat|x".into()]]);
        let lines: Vec<&str> = md.lines().collect();
        assert_eq!(lines[2], r"| feat\|x |");
        assert_eq!(row_cells(lines[2]), 1);
    }

    #[test]
    fn newline_in_a_cell_cannot_break_the_row() {
        let md = table(&["Name"], &[vec!["a\nb".into()]]);
        let lines: Vec<&str> = md.lines().collect();
        assert_eq!(lines.len(), 3, "a newline must not add a row");
        assert_eq!(lines[2], "| a b |");
    }

    #[test]
    fn every_row_has_exactly_the_header_column_count() {
        let headers = ["Project", "Branch", "Status"];
        let rows = vec![
            vec!["a".into(), "main".into(), "clean".into()],
            vec!["b".into(), r"x|y".into(), "dirty".into()],
        ];
        let md = table(&headers, &rows);
        let lines: Vec<&str> = md.lines().collect();
        assert_eq!(row_cells(lines[0]), 3);
        assert_eq!(lines[1].matches("---").count(), 3);
        for line in &lines[2..] {
            assert_eq!(row_cells(line), 3, "{line}");
        }
    }

    #[test]
    fn inline_code_protects_path_characters() {
        assert_eq!(inline_code("/tmp/app_a"), "`/tmp/app_a`");
        // A value containing a backtick needs the longer fence to stay literal.
        assert_eq!(inline_code("a`b"), "`` a`b ``");
        assert_eq!(inline_code("a\nb"), "`a b`");
    }

    #[test]
    fn escapes_only_what_matters() {
        assert_eq!(escape_cell("plain-text_1.0"), "plain-text_1.0");
        assert_eq!(escape_cell("a\r\nb"), "a b");
    }

    /// Count the cells in a rendered row, i.e. the ` |` terminators.
    fn row_cells(line: &str) -> usize {
        line.trim_start_matches('|')
            .split(" |")
            .filter(|s| !s.is_empty())
            .count()
    }
}

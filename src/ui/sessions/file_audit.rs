use std::collections::HashSet;

use crate::locale::t;
use crate::model::{AgentSession, FileOp};
use crate::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::super::truncate_str;

pub(crate) fn draw_file_audit(f: &mut Frame, session: &AgentSession, area: Rect, theme: &Theme) {
    let unique_files: HashSet<&str> = session
        .file_accesses
        .iter()
        .map(|a| a.path.as_str())
        .collect();
    let unique_count = unique_files.len();
    let total_count = session.file_accesses.len();

    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(
        format!(
            " {} ({} accesses, {} unique files)",
            t("detail.file_audit").as_str(),
            total_count,
            unique_count
        ),
        Style::default()
            .fg(theme.title)
            .add_modifier(Modifier::BOLD),
    )));

    let max_rows = area.height.saturating_sub(1) as usize;
    let max_path_w = (area.width as usize).saturating_sub(5);

    for access in session.file_accesses.iter().rev().take(max_rows) {
        let (label, color) = match access.operation {
            FileOp::Read => ("R", theme.session_id),
            FileOp::Edit => ("E", theme.proc_misc),
            FileOp::Write => ("W", theme.cpu_box),
        };
        let max_path = max_path_w.saturating_sub(4);
        let path_display = truncate_str(&access.path, max_path);
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", label), Style::default().fg(color)),
            Span::styled(path_display, Style::default().fg(theme.main_fg)),
            Span::styled(
                format!(" t{}", access.turn_index),
                Style::default().fg(theme.inactive_fg),
            ),
        ]));
    }

    f.render_widget(Paragraph::new(lines), area);
}

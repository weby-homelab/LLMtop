use crate::locale::t;
use crate::model::{AgentSession, ChatRole};
use crate::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::super::truncate_str;

pub(crate) fn draw_chat_history(f: &mut Frame, session: &AgentSession, area: Rect, theme: &Theme) {
    if session.chat_messages.is_empty() {
        return;
    }

    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(
        format!(
            " {} ({})",
            t("detail.chat").as_str(),
            session.chat_messages.len()
        ),
        Style::default()
            .fg(theme.title)
            .add_modifier(Modifier::BOLD),
    )));

    let visible_rows = area.height.saturating_sub(1) as usize;
    let start = session.chat_messages.len().saturating_sub(visible_rows);
    let text_w = (area.width as usize).saturating_sub(6);

    for msg in session.chat_messages.iter().skip(start) {
        let (label, color) = match msg.role {
            ChatRole::User => ("U", theme.hi_fg),
            ChatRole::Assistant => ("A", theme.proc_misc),
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", label), Style::default().fg(color)),
            Span::styled(
                truncate_str(&msg.text, text_w),
                Style::default().fg(theme.main_fg),
            ),
        ]));
    }

    f.render_widget(Paragraph::new(lines), area);
}

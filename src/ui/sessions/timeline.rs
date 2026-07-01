use crate::locale::t;
use crate::model::AgentSession;
use crate::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::super::{truncate_str};

pub(crate) fn draw_timeline(
    f: &mut Frame,
    session: &AgentSession,
    area: Rect,
    theme: &Theme,
    scroll: usize,
) {
    let tool_calls = &session.tool_calls;
    let is_thinking = session.thinking_since_ms > 0
        && matches!(
            session.status,
            crate::model::SessionStatus::Thinking
                | crate::model::SessionStatus::Executing
                | crate::model::SessionStatus::Waiting
                | crate::model::SessionStatus::Unknown
        );
    if tool_calls.is_empty() && !is_thinking {
        return;
    }

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let live_duration = |tc: &crate::model::ToolCall| -> u64 {
        if tc.duration_ms > 0 {
            tc.duration_ms
        } else if session.pending_since_ms > 0 {
            now_ms.saturating_sub(session.pending_since_ms)
        } else {
            0
        }
    };
    let is_pending = |tc: &crate::model::ToolCall| -> bool {
        tc.duration_ms == 0 && session.pending_since_ms > 0
    };
    let thinking_duration = if is_thinking {
        now_ms.saturating_sub(session.thinking_since_ms)
    } else {
        0
    };

    let total_duration: u64 = tool_calls.iter().map(live_duration).sum();
    let max_duration = tool_calls
        .iter()
        .map(live_duration)
        .max()
        .unwrap_or(1)
        .max(1);

    let mut lines = Vec::new();

    let pending_count = tool_calls.iter().filter(|tc| is_pending(tc)).count();
    let mut status_notes: Vec<String> = Vec::new();
    if pending_count > 0 {
        status_notes.push(format!("{} running", pending_count));
    }
    if is_thinking {
        status_notes.push(format!("thinking {}", fmt_duration(thinking_duration)));
    }
    let running_note = if status_notes.is_empty() {
        String::new()
    } else {
        format!(", {}", status_notes.join(", "))
    };
    lines.push(Line::from(vec![Span::styled(
        format!(
            " {} ({} calls, {}{})",
            t("detail.timeline").as_str(),
            tool_calls.len(),
            fmt_duration(total_duration),
            running_note,
        ),
        Style::default()
            .fg(theme.title)
            .add_modifier(Modifier::BOLD),
    )]));

    let bar_width = (area.width as usize).saturating_sub(42).max(5);

    let header_rows = 1;
    let thinking_rows = if is_thinking { 1 } else { 0 };
    let visible_rows = (area.height as usize).saturating_sub(header_rows + thinking_rows);
    let start = scroll.min(tool_calls.len().saturating_sub(visible_rows));

    for tc in tool_calls.iter().skip(start).take(visible_rows) {
        let duration = live_duration(tc);
        let pending = is_pending(tc);
        let bar_fill = if max_duration > 0 {
            ((duration as f64 / max_duration as f64) * bar_width as f64).ceil() as usize
        } else {
            0
        };
        let bar_fill = bar_fill.min(bar_width);
        let bar_empty = bar_width - bar_fill;

        let is_longest = duration == max_duration && max_duration > 0 && !pending;
        let star = if is_longest { " *" } else { "" };

        let color = tool_color(&tc.name, theme);
        let pulse_bright = pending && (now_ms / 500).is_multiple_of(2);
        let name_prefix = if pending { "●" } else { " " };
        let name_style = if pending {
            Style::default().fg(color).add_modifier(if pulse_bright {
                Modifier::BOLD
            } else {
                Modifier::DIM
            })
        } else {
            Style::default().fg(color).add_modifier(Modifier::BOLD)
        };
        let bar_style = if pending {
            Style::default().fg(color).add_modifier(Modifier::DIM)
        } else {
            Style::default().fg(color)
        };

        let duration_label = if pending {
            format!(" {:>5}…", fmt_duration(duration))
        } else {
            format!(" {:>6}{}", fmt_duration(duration), star)
        };
        let duration_color = if is_longest {
            theme.proc_misc
        } else if pending {
            color
        } else {
            theme.graph_text
        };

        let name_label = truncate_str(tool_label(&tc.name), 6);
        lines.push(Line::from(vec![
            Span::styled(format!("{}{:<6}", name_prefix, name_label), name_style),
            Span::styled(
                format!(" {:<20}", truncate_str(&tc.arg, 20)),
                Style::default().fg(theme.graph_text),
            ),
            Span::styled(" ", Style::default()),
            Span::styled("█".repeat(bar_fill), bar_style),
            Span::styled("░".repeat(bar_empty), Style::default().fg(theme.div_line)),
            Span::styled(duration_label, Style::default().fg(duration_color)),
        ]));
    }

    if is_thinking {
        let color = theme.title;
        let pulse_bright = (now_ms / 500).is_multiple_of(2);
        let bar_fill = if max_duration > 0 {
            ((thinking_duration as f64 / max_duration as f64) * bar_width as f64).ceil() as usize
        } else {
            bar_width
        };
        let bar_fill = bar_fill.min(bar_width);
        let bar_empty = bar_width - bar_fill;
        let name_style = Style::default().fg(color).add_modifier(if pulse_bright {
            Modifier::BOLD
        } else {
            Modifier::DIM
        });
        let bar_style = Style::default().fg(color).add_modifier(Modifier::DIM);
        lines.push(Line::from(vec![
            Span::styled("●Think ", name_style),
            Span::styled(
                format!(" {:<20}", "generating reply"),
                Style::default().fg(theme.graph_text),
            ),
            Span::styled(" ", Style::default()),
            Span::styled("█".repeat(bar_fill), bar_style),
            Span::styled("░".repeat(bar_empty), Style::default().fg(theme.div_line)),
            Span::styled(
                format!(" {:>5}…", fmt_duration(thinking_duration)),
                Style::default().fg(color),
            ),
        ]));
    }

    f.render_widget(Paragraph::new(lines), area);
}

pub(crate) fn tool_color(name: &str, theme: &Theme) -> Color {
    match name {
        "Read" => theme.session_id,
        "Edit" => theme.proc_misc,
        "Write" => theme.cpu_box,
        "Bash" => theme.hi_fg,
        "shell" | "exec_command" | "write_stdin" => theme.hi_fg,
        "apply_patch" => theme.proc_misc,
        "update_plan" => theme.title,
        "spawn_agent" | "send_input" | "wait_agent" => theme.title,
        "view_image" => theme.session_id,
        "Grep" => theme.status_fg,
        "Glob" => theme.graph_text,
        "find" | "list_mcp_resources" | "read_mcp_resource" => theme.status_fg,
        "Agent" => theme.title,
        "Skill" => theme.selected_fg,
        _ => theme.inactive_fg,
    }
}

pub(crate) fn tool_label(name: &str) -> &str {
    match name {
        "exec_command" | "shell" => "Exec",
        "write_stdin" => "Input",
        "apply_patch" => "Patch",
        "update_plan" => "Plan",
        "spawn_agent" => "Agent",
        "send_input" => "Send",
        "wait_agent" => "Wait",
        "view_image" => "Image",
        "list_mcp_resources" | "read_mcp_resource" => "MCP",
        other => other,
    }
}

fn fmt_duration(ms: u64) -> String {
    if ms >= 60_000 {
        format!("{}m{:.0}s", ms / 60_000, (ms % 60_000) as f64 / 1000.0)
    } else if ms >= 1000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        format!("{}ms", ms)
    }
}

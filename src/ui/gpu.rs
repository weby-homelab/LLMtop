use crate::app::App;
use crate::locale::t;
use crate::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::{btop_block_active, grad_at, make_gradient};

pub(crate) fn draw_gpu_panel(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    draw_gpu_panel_active(f, app, area, theme, false);
}

pub(crate) fn draw_gpu_panel_active(
    f: &mut Frame,
    app: &App,
    area: Rect,
    theme: &Theme,
    active: bool,
) {
    let cpu_grad = make_gradient(theme.cpu_grad.start, theme.cpu_grad.mid, theme.cpu_grad.end);

    let block = btop_block_active("gpu", "▣", theme.cpu_box, theme, active);
    f.render_widget(block, area);

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    if app.gpu_metrics.is_empty() {
        let label = t("gpu.no_gpu");
        f.render_widget(
            Paragraph::new(vec![Line::from(Span::styled(
                format!("  {label}"),
                Style::default().fg(theme.inactive_fg),
            ))]),
            inner,
        );
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    for gpu in &app.gpu_metrics {
        let vram_pct = if gpu.vram_total_mb > 0 {
            (gpu.vram_used_mb as f64 / gpu.vram_total_mb as f64) * 100.0
        } else {
            0.0
        };
        let temp_color = if gpu.temp_c > 85 {
            let (r, g, b) = theme.cpu_grad.end;
            Color::Rgb(r, g, b)
        } else if gpu.temp_c > 65 {
            let (r, g, b) = theme.cpu_grad.mid;
            Color::Rgb(r, g, b)
        } else {
            let (r, g, b) = theme.cpu_grad.start;
            Color::Rgb(r, g, b)
        };
        let util_color = grad_at(&cpu_grad, gpu.utilization_pct as f64);

        let name_short = if gpu.name.len() > 16 {
            &gpu.name[..16]
        } else {
            &gpu.name
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!(" GPU{} ", gpu.index),
                Style::default().fg(theme.title).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{name_short:<16} "),
                Style::default().fg(theme.main_fg),
            ),
            Span::styled(
                format!("{:>3}°C ", gpu.temp_c),
                Style::default().fg(temp_color),
            ),
            Span::styled(
                format!("{:>3}% ", gpu.utilization_pct),
                Style::default().fg(util_color),
            ),
            Span::styled(
                format!("{}M/{:>5}M ", gpu.vram_used_mb, gpu.vram_total_mb),
                Style::default().fg(grad_at(&cpu_grad, vram_pct)),
            ),
            Span::styled(
                format!("{:>3}W/{:>3}W", gpu.power_w, gpu.power_max_w),
                Style::default().fg(theme.graph_text),
            ),
        ]));
    }

    f.render_widget(Paragraph::new(lines), inner);
}

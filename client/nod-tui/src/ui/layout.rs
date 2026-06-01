use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub(super) fn render_box(
    frame: &mut Frame<'_>,
    area: Rect,
    title: &str,
    body: Vec<Line<'_>>,
    width: u16,
    height: u16,
) {
    let modal_area = centered_rect(width, height, area);
    frame.render_widget(Clear, modal_area);
    frame.render_widget(
        Paragraph::new(body)
            .block(Block::default().title(title).borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
        modal_area,
    );
}

pub(super) fn focused_block(title: &str, focused: bool) -> Block<'_> {
    let style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };
    Block::default()
        .title(title.to_string())
        .borders(Borders::ALL)
        .border_style(style)
}

pub(super) fn centered_inner(width: u16, height: u16, area: Rect) -> Rect {
    let outer = centered_rect(width, height, area);
    Rect {
        x: outer.x.saturating_add(2),
        y: outer.y.saturating_add(1),
        width: outer.width.saturating_sub(4),
        height: outer.height.saturating_sub(2),
    }
}

pub(super) fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

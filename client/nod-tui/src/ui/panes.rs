use nod_client_core::models::{Channel, Event, EventStatus};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::{
    app::{AppState, EnrollmentField, EnrollmentForm, Focus, Modal},
    domain,
};

use super::{
    format::{action_key_hint, event_status_label, form_line, selected_marker, status_label},
    layout::{centered_inner, centered_rect, focused_block},
};

pub(super) fn render_main(frame: &mut Frame<'_>, area: Rect, app: &AppState) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(area);
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(28),
            Constraint::Percentage(36),
            Constraint::Percentage(64),
        ])
        .split(vertical[0]);

    render_sidebar(frame, columns[0], app);
    render_event_list(frame, columns[1], app);
    render_detail(frame, columns[2], app);
    render_status(frame, vertical[1], app);
}

pub(super) fn render_enrollment(frame: &mut Frame<'_>, area: Rect, app: &AppState) {
    let block = Block::default().title("Nod").borders(Borders::ALL);
    frame.render_widget(block, centered_rect(56, 12, area));
    if let Some(Modal::Enrollment(form)) = app.modal() {
        render_enrollment_form(
            frame,
            centered_inner(56, 12, area),
            form,
            app.error().or_else(|| app.running()),
        );
    }
}

fn render_sidebar(frame: &mut Frame<'_>, area: Rect, app: &AppState) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(8)])
        .split(area);
    render_servers(frame, sections[0], app);
    render_channels(frame, sections[1], app);
}

fn render_servers(frame: &mut Frame<'_>, area: Rect, app: &AppState) {
    let selected_id = domain::selected_server_id(app.client_state());
    let items: Vec<ListItem<'_>> = app
        .client_state()
        .servers
        .iter()
        .map(|server| {
            let marker = selected_marker(selected_id == Some(server.id.as_str()));
            ListItem::new(Line::from(format!("{marker}{}", server.name)))
        })
        .collect();

    frame.render_widget(
        List::new(items)
            .block(focused_block("Servers", app.focus() == Focus::Servers))
            .style(Style::default().fg(Color::White)),
        area,
    );
}

fn render_channels(frame: &mut Frame<'_>, area: Rect, app: &AppState) {
    let selected_id =
        domain::selected_channel(app.client_state()).map(|channel| channel.id.as_str());
    let items: Vec<ListItem<'_>> = domain::subscribed_channels(app.client_state())
        .iter()
        .map(|channel| channel_item(channel, selected_id, app))
        .collect();

    frame.render_widget(
        List::new(items).block(focused_block("Channels", app.focus() == Focus::Channels)),
        area,
    );
}

fn channel_item<'a>(channel: &Channel, selected_id: Option<&str>, app: &AppState) -> ListItem<'a> {
    let marker = selected_marker(selected_id == Some(channel.id.as_str()));
    let count = domain::pending_count_for(channel, app.client_state());
    ListItem::new(Line::from(format!("{marker}{} ({count})", channel.name)))
}

fn render_event_list(frame: &mut Frame<'_>, area: Rect, app: &AppState) {
    let selected_id = domain::selected_event(app.client_state()).map(|event| event.id.as_str());
    let items: Vec<ListItem<'_>> = app
        .visible_events()
        .into_iter()
        .map(|event| event_item(event, selected_id))
        .collect();
    let title = if app.filter().is_empty() {
        "Notifications".to_string()
    } else {
        format!("Notifications /{}", app.filter())
    };

    let empty = if items.is_empty() {
        vec![ListItem::new("No notifications")]
    } else {
        items
    };
    frame.render_widget(
        List::new(empty).block(focused_block(&title, app.focus() == Focus::Events)),
        area,
    );
}

fn event_item<'a>(event: &Event, selected_id: Option<&str>) -> ListItem<'a> {
    let marker = selected_marker(selected_id == Some(event.id.as_str()));
    let status = event_status_label(&event.status);
    let summary = if event.summary.is_empty() {
        event.body_markdown.as_str()
    } else {
        event.summary.as_str()
    };
    ListItem::new(vec![
        Line::from(format!("{marker}{} [{status}]", event.title)),
        Line::from(Span::styled(
            format!("  {summary}"),
            Style::default().fg(Color::DarkGray),
        )),
    ])
}

fn render_detail(frame: &mut Frame<'_>, area: Rect, app: &AppState) {
    let Some(event) = domain::selected_event(app.client_state()) else {
        frame.render_widget(
            Paragraph::new("Select a notification")
                .block(focused_block("Detail", app.focus() == Focus::Detail)),
            area,
        );
        return;
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled(&event.title, Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("  {}", status_label(event))),
        ]),
        Line::from(event.summary.clone()),
        Line::from(""),
    ];
    if !event.body_markdown.is_empty() {
        lines.push(Line::from(event.body_markdown.clone()));
        lines.push(Line::from(""));
    }
    for field in &event.fields {
        lines.push(Line::from(format!("{}: {}", field.label, field.value)));
    }
    if !event.links.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from("Links"));
        for link in &event.links {
            lines.push(Line::from(format!("{} - {}", link.label, link.url)));
        }
    }
    if event.status == EventStatus::Pending {
        lines.push(Line::from(""));
        lines.push(Line::from("Actions"));
        if event.actions.is_empty() {
            lines.push(Line::from("d dismiss"));
        } else {
            for action in &event.actions {
                lines.push(Line::from(format!(
                    "{}  {}",
                    action_key_hint(action.kind.as_str()),
                    action.label
                )));
            }
        }
    }

    frame.render_widget(
        Paragraph::new(lines)
            .block(focused_block("Detail", app.focus() == Focus::Detail))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_status(frame: &mut Frame<'_>, area: Rect, app: &AppState) {
    let pending = domain::total_pending_count(app.client_state());
    let sync = if app.client_state().is_sync_connected {
        "sync:on"
    } else {
        "sync:off"
    };
    let alerts = if app.alerts().muted() {
        "alerts:muted"
    } else {
        "alerts:on"
    };
    let message = app
        .error()
        .or_else(|| app.running())
        .or_else(|| app.alerts().message())
        .unwrap_or_else(|| app.status());
    let style = if app.error().is_some() {
        Style::default().fg(Color::Red)
    } else if app.alerts().flashing() {
        Style::default().fg(Color::Black).bg(Color::Yellow)
    } else {
        Style::default().fg(Color::Gray)
    };
    let text = format!("pending:{pending}  {sync}  {alerts}  {message}");
    frame.render_widget(Paragraph::new(text).style(style), area);
}

fn render_enrollment_form(
    frame: &mut Frame<'_>,
    area: Rect,
    form: &EnrollmentForm,
    error: Option<&str>,
) {
    let lines = vec![
        form_line(
            "Server",
            form.base_url().value(),
            form.active_field() == EnrollmentField::Server,
        ),
        form_line(
            "Device",
            form.device_name().value(),
            form.active_field() == EnrollmentField::Device,
        ),
        form_line(
            "Code",
            form.code().value(),
            form.active_field() == EnrollmentField::Code,
        ),
        form_line(
            "Sound",
            form.selected_sound(),
            form.active_field() == EnrollmentField::Sound,
        ),
        Line::from(""),
        Line::from(error.unwrap_or("Enter to enroll. Tab moves fields.")),
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

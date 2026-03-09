use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, FocusPane};

pub fn render(frame: &mut Frame, app: &App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(frame.area());

    render_header(frame, app, root[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(root[1]);

    render_packet_list(frame, app, body[0]);
    render_packet_detail(frame, app, body[1]);
    render_filter_bar(frame, app, root[2]);
    render_footer(frame, root[3]);
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let status = format!(
        "tcpdump-tui M1 skeleton | capture: {:?} | packets: {}",
        app.capture_state(),
        app.packets().len()
    );

    let header =
        Paragraph::new(status).block(Block::default().borders(Borders::ALL).title("Status"));
    frame.render_widget(header, area);
}

fn render_packet_list(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .packets()
        .iter()
        .map(|packet| {
            ListItem::new(Line::raw(format!(
                "{} {} -> {} {} {}B",
                packet.timestamp, packet.source, packet.destination, packet.protocol, packet.length
            )))
        })
        .collect();

    let list = List::new(items)
        .block(focused_block(
            "Packets",
            app.focus() == FocusPane::PacketList,
        ))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    let mut state = ListState::default();
    if !app.packets().is_empty() {
        state.select(Some(app.selected_packet_index()));
    }

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_packet_detail(frame: &mut Frame, app: &App, area: Rect) {
    let lines = if let Some(packet) = app.selected_packet() {
        vec![
            Line::raw(format!("timestamp: {}", packet.timestamp)),
            Line::raw(format!("source: {}", packet.source)),
            Line::raw(format!("destination: {}", packet.destination)),
            Line::raw(format!("protocol: {}", packet.protocol)),
            Line::raw(format!("length: {} bytes", packet.length)),
            Line::raw(""),
            Line::raw("summary:"),
            Line::raw(packet.summary.as_str()),
        ]
    } else {
        vec![Line::raw("No packet selected")]
    };

    let detail = Paragraph::new(lines)
        .block(focused_block(
            "Packet Detail",
            app.focus() == FocusPane::PacketDetail,
        ))
        .wrap(Wrap { trim: true });

    frame.render_widget(detail, area);
}

fn render_filter_bar(frame: &mut Frame, app: &App, area: Rect) {
    let filter = Paragraph::new(app.filter_input()).block(focused_block(
        "Filter (placeholder)",
        app.focus() == FocusPane::FilterInput,
    ));

    frame.render_widget(filter, area);
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let footer =
        Paragraph::new("q: quit | j/down: next packet | k/up: previous packet | tab: cycle focus");
    frame.render_widget(footer, area);
}

fn focused_block(title: &str, is_focused: bool) -> Block<'_> {
    let border_style = if is_focused {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border_style)
}

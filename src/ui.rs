use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, FocusPane};
use crate::domain::{tcp_packet_details, PacketSummary, TcpPacketDetails};

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
        .constraints([
            Constraint::Length(18),
            Constraint::Percentage(52),
            Constraint::Percentage(48),
        ])
        .split(root[1]);

    render_filter_selector(frame, app, body[0]);
    render_packet_list(frame, app, body[1]);
    render_packet_detail(frame, app, body[2]);
    render_filter_bar(frame, app, root[2]);
    render_footer(frame, root[3]);
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let status = format!(
        "lazytcp | capture: {:?} | packets: {}",
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

    let title = format!("Packets [{}]", app.packets().len());
    let list = List::new(items)
        .block(focused_block(&title, app.focus() == FocusPane::PacketList))
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

fn render_filter_selector(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .filter_dimensions()
        .iter()
        .map(|dimension| ListItem::new(Line::raw(dimension.as_str())))
        .collect();

    let list = List::new(items)
        .block(focused_block(
            "Filter Type",
            app.focus() == FocusPane::FilterSelector,
        ))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    let mut state = ListState::default();
    if !app.filter_dimensions().is_empty() {
        state.select(Some(app.selected_filter_dimension_index()));
    }

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_packet_detail(frame: &mut Frame, app: &App, area: Rect) {
    let block = focused_block("Packet Detail", app.focus() == FocusPane::PacketDetail);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    if let Some(packet) = app.selected_packet() {
        render_packet_detail_component(frame, packet, inner);
    } else {
        frame.render_widget(Paragraph::new("No packet selected"), inner);
    }
}

fn render_packet_detail_component(frame: &mut Frame, packet: &PacketSummary, area: Rect) {
    if let Some(details) = tcp_packet_details(packet) {
        render_tcp_packet_visualizer(frame, packet, &details, area);
    } else {
        render_default_packet_detail(frame, packet, area);
    }
}

fn render_tcp_packet_visualizer(
    frame: &mut Frame,
    packet: &PacketSummary,
    details: &TcpPacketDetails,
    area: Rect,
) {
    if area_too_small_for_tcp_layout(area) {
        render_default_packet_detail(frame, packet, area);
        return;
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(3),
        ])
        .split(area);

    let ports_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);

    render_sub_pane(
        frame,
        ports_row[0],
        "Source Port",
        format_option(details.source_port.map(|port| port.to_string())),
    );
    render_sub_pane(
        frame,
        ports_row[1],
        "Destination Port",
        format_option(details.destination_port.map(|port| port.to_string())),
    );

    render_sub_pane(
        frame,
        rows[1],
        "Sequence Number",
        format_option(details.sequence_number.as_deref().map(str::to_string)),
    );

    render_sub_pane(
        frame,
        rows[2],
        "Acknowledgment Number",
        format_option(
            details
                .acknowledgement_number
                .as_deref()
                .map(str::to_string),
        ),
    );

    let control_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(13),
            Constraint::Percentage(9),
            Constraint::Percentage(28),
            Constraint::Percentage(50),
        ])
        .split(rows[3]);

    render_sub_pane(
        frame,
        control_row[0],
        "Data Offset",
        format_option(
            details
                .data_offset_words
                .map(|words| format!("{words} words / {}B", words as usize * 4)),
        ),
    );
    render_sub_pane(
        frame,
        control_row[1],
        "Reserved",
        format_option(details.reserved_bits.as_deref().map(str::to_string)),
    );

    render_sub_pane(
        frame,
        control_row[2],
        "Flags",
        format!(
            "N{} C{} E{} U{} A{} P{} R{} S{} F{} [{}]",
            bit(details.flags.ns),
            bit(details.flags.cwr),
            bit(details.flags.ece),
            bit(details.flags.urg),
            bit(details.flags.ack),
            bit(details.flags.psh),
            bit(details.flags.rst),
            bit(details.flags.syn),
            bit(details.flags.fin),
            if details.flags.raw.is_empty() {
                "-"
            } else {
                &details.flags.raw
            }
        ),
    );

    render_sub_pane(
        frame,
        control_row[3],
        "Window Size",
        format_option(details.window_size.map(|size| size.to_string())),
    );

    let checksum_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[4]);

    render_sub_pane(
        frame,
        checksum_row[0],
        "Checksum",
        format_option(details.checksum.as_deref().map(str::to_string)),
    );
    render_sub_pane(
        frame,
        checksum_row[1],
        "Urgent Pointer",
        format_option(details.urgent_pointer.map(|pointer| pointer.to_string())),
    );

    render_sub_pane(
        frame,
        rows[5],
        "Options",
        format_option(details.options.as_deref().map(str::to_string)),
    );

    render_sub_pane(
        frame,
        rows[6],
        "Data / Payload",
        format!(
            "{} -> {}\nlen: {} bytes\nsummary: {}",
            packet.source, packet.destination, details.payload_length, packet.summary
        ),
    );
}

fn render_default_packet_detail(frame: &mut Frame, packet: &PacketSummary, area: Rect) {
    let lines = vec![
        Line::raw(format!("timestamp: {}", packet.timestamp)),
        Line::raw(format!("source: {}", packet.source)),
        Line::raw(format!("destination: {}", packet.destination)),
        Line::raw(format!("protocol: {}", packet.protocol)),
        Line::raw(format!("length: {} bytes", packet.length)),
        Line::raw(""),
        Line::raw("summary:"),
        Line::raw(packet.summary.clone()),
    ];

    let detail = Paragraph::new(lines).wrap(Wrap { trim: true });
    frame.render_widget(detail, area);
}

fn render_sub_pane(frame: &mut Frame, area: Rect, title: &str, value: impl Into<String>) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let pane = Paragraph::new(value.into())
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: true });
    frame.render_widget(pane, area);
}

fn area_too_small_for_tcp_layout(area: Rect) -> bool {
    area.width < 36 || area.height < 21
}

fn format_option(value: Option<String>) -> String {
    value.unwrap_or_else(|| "-".to_string())
}

fn bit(value: bool) -> u8 {
    if value {
        1
    } else {
        0
    }
}

fn render_filter_bar(frame: &mut Frame, app: &App, area: Rect) {
    let filter_display = format!(
        "{}: {}",
        app.selected_filter_dimension().as_str(),
        app.filter_input()
    );

    let filter = Paragraph::new(filter_display).block(focused_block(
        "Filter Expression",
        app.focus() == FocusPane::FilterInput,
    ));

    frame.render_widget(filter, area);

    if app.focus() == FocusPane::FilterInput {
        if let Some((x, y)) = filter_cursor_position(area, app) {
            frame.set_cursor_position((x, y));
        }
    }
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(
        "q: quit | j/k or arrows: move selection in focused list | enter: filter type -> expression -> packets | tab/shift+tab: cycle focus",
    );
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

fn filter_cursor_position(area: Rect, app: &App) -> Option<(u16, u16)> {
    let inner = area.inner(ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    });
    if inner.width == 0 || inner.height == 0 {
        return None;
    }

    let prefix = format!("{}: ", app.selected_filter_dimension().as_str());
    let cursor_col = prefix.chars().count() + app.filter_input().chars().count();

    let max_col = inner.width.saturating_sub(1) as usize;
    let clamped_col = cursor_col.min(max_col) as u16;
    Some((inner.x.saturating_add(clamped_col), inner.y))
}

#[cfg(test)]
mod tests {
    use super::filter_cursor_position;
    use crate::app::App;
    use ratatui::layout::Rect;

    #[test]
    fn filter_cursor_position_is_inside_filter_bar_inner_area() {
        let mut app = App::new();
        app.focus_filter_input();
        app.insert_filter_input_char('u');
        app.insert_filter_input_char('d');
        app.insert_filter_input_char('p');

        let area = Rect::new(0, 0, 30, 3);
        let (x, y) = filter_cursor_position(area, &app)
            .expect("cursor should be available for a valid filter area");

        assert!((1..=28).contains(&x));
        assert_eq!(y, 1);
    }

    #[test]
    fn filter_cursor_position_returns_none_when_area_too_small() {
        let app = App::new();
        assert!(filter_cursor_position(Rect::new(0, 0, 1, 1), &app).is_none());
    }
}

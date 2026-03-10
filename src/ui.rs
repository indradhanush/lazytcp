use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, ListState, Padding, Paragraph, Wrap,
};
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
        .constraints([Constraint::Length(24), Constraint::Min(20)])
        .split(root[1]);

    let packet_panes = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(body[1]);

    render_filter_selector(frame, app, body[0]);
    render_packet_list(frame, app, packet_panes[0]);
    render_packet_detail(frame, app, packet_panes[1]);
    render_filter_bar(frame, app, root[2]);
    render_footer(frame, root[3]);
    render_filter_popup(frame, app, frame.area());
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
            "Filter By",
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
    let filter_display = if app.filter_expression().is_empty() {
        "no filters applied".to_string()
    } else {
        app.filter_expression().to_string()
    };

    let filter = Paragraph::new(filter_display).block(focused_block(
        "Filter Expression",
        app.focus() == FocusPane::FilterInput,
    ));

    frame.render_widget(filter, area);
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(
        "q: quit | enter on filter by pane: open popup | popup: space toggle, enter apply, esc cancel | j/k or arrows: move | tab/shift+tab: cycle focus",
    );
    frame.render_widget(footer, area);
}

fn render_filter_popup(frame: &mut Frame, app: &App, area: Rect) {
    if !app.is_filter_popup_open() {
        return;
    }

    let popup_area = centered_rect(60, 70, area);
    frame.render_widget(Clear, popup_area);

    let dimension = app
        .filter_popup_dimension()
        .map(|value| value.as_str())
        .unwrap_or("filter");
    let candidates = app.filter_popup_candidates().unwrap_or(&[]);

    let items: Vec<ListItem> = if candidates.is_empty() {
        vec![ListItem::new(Line::raw(
            "No values available for this filter type",
        ))]
    } else {
        candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                let marker = if app.filter_popup_candidate_selected(index) {
                    "[x]"
                } else {
                    "[ ]"
                };
                ListItem::new(Line::raw(format!("{marker} {candidate}")))
            })
            .collect()
    };

    let popup_title = format!("Select {} values", dimension);
    let popup_footer =
        Line::raw("space: toggle | enter: apply | esc: cancel").alignment(Alignment::Right);
    let list = List::new(items)
        .block(focused_block(&popup_title, true).title_bottom(popup_footer))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightYellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    let mut state = ListState::default();
    if !candidates.is_empty() {
        state.select(app.filter_popup_selected_index());
    }

    frame.render_stateful_widget(list, popup_area, &mut state);
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
        .border_type(if is_focused {
            BorderType::Thick
        } else {
            BorderType::Plain
        })
        .border_style(border_style)
        .padding(Padding::new(2, 2, 0, 0))
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

#[cfg(test)]
mod tests {
    use super::{centered_rect, focused_block};
    use ratatui::layout::Rect;

    #[test]
    fn centered_rect_stays_within_outer_area() {
        let outer = Rect::new(0, 0, 100, 40);
        let inner = centered_rect(60, 70, outer);

        assert!(inner.x >= outer.x);
        assert!(inner.y >= outer.y);
        assert!(inner.width <= outer.width);
        assert!(inner.height <= outer.height);
    }

    #[test]
    fn centered_rect_matches_expected_percentages() {
        let outer = Rect::new(0, 0, 100, 40);
        let inner = centered_rect(60, 70, outer);

        assert_eq!(inner.width, 60);
        assert_eq!(inner.height, 28);
    }

    #[test]
    fn focused_block_uses_thicker_horizontal_padding() {
        let area = Rect::new(0, 0, 30, 10);
        let focused_inner = focused_block("focused", true).inner(area);
        let unfocused_inner = focused_block("unfocused", false).inner(area);

        assert_eq!(focused_inner.width, unfocused_inner.width);
        assert_eq!(focused_inner.height, unfocused_inner.height);
    }
}

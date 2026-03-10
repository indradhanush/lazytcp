use std::collections::BTreeSet;

use crate::capture::CaptureState;
use crate::domain::{FilterDimension, PacketSummary};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPane {
    FilterSelector,
    PacketList,
    FilterInput,
    PacketDetail,
}

#[derive(Debug, Clone)]
struct FilterPopup {
    dimension: FilterDimension,
    candidates: Vec<String>,
    selected_values: BTreeSet<String>,
    highlighted_index: usize,
}

pub struct App {
    should_quit: bool,
    focus: FocusPane,
    all_packets: Vec<PacketSummary>,
    packets: Vec<PacketSummary>,
    selected_packet: usize,
    filter_dimensions: Vec<FilterDimension>,
    selected_filter_dimension: usize,
    active_filter_values_by_dimension: Vec<Vec<String>>,
    filter_expression: String,
    capture_state: CaptureState,
    filter_popup: Option<FilterPopup>,
}

impl App {
    pub fn new() -> Self {
        Self::with_packets(Vec::new(), String::new())
    }

    pub fn with_packets(packets: Vec<PacketSummary>, _filter_input: String) -> Self {
        let filter_dimensions = FilterDimension::ALL.to_vec();
        let mut app = Self {
            should_quit: false,
            focus: FocusPane::FilterSelector,
            all_packets: packets.clone(),
            packets,
            selected_packet: 0,
            filter_dimensions: filter_dimensions.clone(),
            selected_filter_dimension: 0,
            active_filter_values_by_dimension: vec![Vec::new(); filter_dimensions.len()],
            filter_expression: String::new(),
            capture_state: CaptureState::Idle,
            filter_popup: None,
        };
        app.apply_active_filter();
        app
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn packets(&self) -> &[PacketSummary] {
        &self.packets
    }

    pub fn selected_packet_index(&self) -> usize {
        self.selected_packet
    }

    pub fn selected_packet(&self) -> Option<&PacketSummary> {
        self.packets.get(self.selected_packet)
    }

    pub fn filter_dimensions(&self) -> &[FilterDimension] {
        &self.filter_dimensions
    }

    pub fn selected_filter_dimension_index(&self) -> usize {
        self.selected_filter_dimension
    }

    pub fn selected_filter_dimension(&self) -> FilterDimension {
        self.filter_dimensions
            .get(self.selected_filter_dimension)
            .copied()
            .unwrap_or(FilterDimension::Host)
    }

    pub fn filter_expression(&self) -> &str {
        &self.filter_expression
    }

    pub fn is_filter_popup_open(&self) -> bool {
        self.filter_popup.is_some()
    }

    pub fn filter_popup_dimension(&self) -> Option<FilterDimension> {
        self.filter_popup.as_ref().map(|popup| popup.dimension)
    }

    pub fn filter_popup_candidates(&self) -> Option<&[String]> {
        self.filter_popup
            .as_ref()
            .map(|popup| popup.candidates.as_slice())
    }

    pub fn filter_popup_selected_index(&self) -> Option<usize> {
        self.filter_popup
            .as_ref()
            .map(|popup| popup.highlighted_index)
    }

    pub fn filter_popup_candidate_selected(&self, index: usize) -> bool {
        let Some(popup) = self.filter_popup.as_ref() else {
            return false;
        };

        let Some(candidate) = popup.candidates.get(index) else {
            return false;
        };

        popup.selected_values.contains(candidate)
    }

    pub fn open_filter_popup(&mut self) {
        let dimension = self.selected_filter_dimension();
        let candidates = filter_candidates(&self.all_packets, dimension);

        let selected_values: BTreeSet<String> = self
            .active_filter_values_for_selected_dimension()
            .iter()
            .cloned()
            .collect();
        let highlighted_index = candidates
            .iter()
            .position(|candidate| selected_values.contains(candidate))
            .unwrap_or(0);

        self.filter_popup = Some(FilterPopup {
            dimension,
            candidates,
            selected_values,
            highlighted_index,
        });
    }

    pub fn close_filter_popup(&mut self) {
        self.filter_popup = None;
    }

    pub fn confirm_filter_popup(&mut self) {
        let Some(popup) = self.filter_popup.take() else {
            return;
        };

        let selected_values: Vec<String> = popup
            .candidates
            .iter()
            .filter(|candidate| popup.selected_values.contains(*candidate))
            .cloned()
            .collect();
        self.set_active_filter_values(popup.dimension, selected_values);
        self.filter_expression = build_filter_expression(
            &self.filter_dimensions,
            &self.active_filter_values_by_dimension,
        );
        self.apply_active_filter();
    }

    pub fn toggle_filter_popup_selection(&mut self) {
        let Some(popup) = self.filter_popup.as_mut() else {
            return;
        };

        let Some(candidate) = popup.candidates.get(popup.highlighted_index).cloned() else {
            return;
        };

        if !popup.selected_values.insert(candidate.clone()) {
            popup.selected_values.remove(&candidate);
        }
    }

    pub fn focus_filter_input(&mut self) {
        self.focus = FocusPane::FilterInput;
    }

    pub fn focus_packet_list(&mut self) {
        self.focus = FocusPane::PacketList;
    }

    pub fn focus(&self) -> FocusPane {
        self.focus
    }

    pub fn capture_state(&self) -> CaptureState {
        self.capture_state
    }

    pub fn next_packet(&mut self) {
        if self.packets.is_empty() {
            return;
        }
        self.selected_packet = (self.selected_packet + 1).min(self.packets.len() - 1);
    }

    pub fn previous_packet(&mut self) {
        self.selected_packet = self.selected_packet.saturating_sub(1);
    }

    pub fn next_filter_dimension(&mut self) {
        if self.filter_dimensions.is_empty() {
            return;
        }

        let next_index = (self.selected_filter_dimension + 1).min(self.filter_dimensions.len() - 1);
        self.update_filter_dimension(next_index);
    }

    pub fn previous_filter_dimension(&mut self) {
        let previous_index = self.selected_filter_dimension.saturating_sub(1);
        self.update_filter_dimension(previous_index);
    }

    fn update_filter_dimension(&mut self, new_index: usize) {
        if self.selected_filter_dimension == new_index {
            return;
        }

        self.selected_filter_dimension = new_index;
        self.filter_popup = None;
    }

    pub fn move_down(&mut self) {
        if let Some(popup) = self.filter_popup.as_mut() {
            if popup.candidates.is_empty() {
                return;
            }
            popup.highlighted_index = (popup.highlighted_index + 1) % popup.candidates.len();
            return;
        }

        match self.focus {
            FocusPane::FilterSelector => self.next_filter_dimension(),
            FocusPane::PacketList => self.next_packet(),
            FocusPane::FilterInput | FocusPane::PacketDetail => {}
        }
    }

    pub fn move_up(&mut self) {
        if let Some(popup) = self.filter_popup.as_mut() {
            if popup.candidates.is_empty() {
                return;
            }
            popup.highlighted_index = if popup.highlighted_index == 0 {
                popup.candidates.len() - 1
            } else {
                popup.highlighted_index - 1
            };
            return;
        }

        match self.focus {
            FocusPane::FilterSelector => self.previous_filter_dimension(),
            FocusPane::PacketList => self.previous_packet(),
            FocusPane::FilterInput | FocusPane::PacketDetail => {}
        }
    }

    pub fn cycle_focus(&mut self) {
        if self.filter_popup.is_some() {
            return;
        }

        self.focus = match self.focus {
            FocusPane::FilterSelector => FocusPane::PacketList,
            FocusPane::PacketList => FocusPane::PacketDetail,
            FocusPane::PacketDetail => FocusPane::FilterSelector,
            FocusPane::FilterInput => FocusPane::PacketList,
        };
    }

    pub fn reverse_cycle_focus(&mut self) {
        if self.filter_popup.is_some() {
            return;
        }

        self.focus = match self.focus {
            FocusPane::FilterSelector => FocusPane::PacketDetail,
            FocusPane::FilterInput => FocusPane::FilterSelector,
            FocusPane::PacketDetail => FocusPane::PacketList,
            FocusPane::PacketList => FocusPane::FilterSelector,
        };
    }

    fn apply_active_filter(&mut self) {
        if self
            .active_filter_values_by_dimension
            .iter()
            .all(|values| values.is_empty())
        {
            self.packets = self.all_packets.clone();
            self.clamp_selected_packet();
            return;
        }

        self.packets = self
            .all_packets
            .iter()
            .filter(|packet| packet_matches_all_active_filters(self, packet))
            .cloned()
            .collect();
        self.clamp_selected_packet();
    }

    fn active_filter_values_for_selected_dimension(&self) -> &[String] {
        self.active_filter_values_by_dimension
            .get(self.selected_filter_dimension)
            .map(|values| values.as_slice())
            .unwrap_or(&[])
    }

    fn set_active_filter_values(&mut self, dimension: FilterDimension, values: Vec<String>) {
        if let Some(index) = self
            .filter_dimensions
            .iter()
            .position(|candidate| *candidate == dimension)
        {
            if let Some(slot) = self.active_filter_values_by_dimension.get_mut(index) {
                *slot = values;
            }
        }
    }

    fn clamp_selected_packet(&mut self) {
        if self.packets.is_empty() {
            self.selected_packet = 0;
            return;
        }

        self.selected_packet = self.selected_packet.min(self.packets.len() - 1);
    }
}

fn build_filter_expression(
    dimensions: &[FilterDimension],
    values_by_dimension: &[Vec<String>],
) -> String {
    let mut clauses = Vec::new();

    for (index, dimension) in dimensions.iter().enumerate() {
        let Some(values) = values_by_dimension.get(index) else {
            continue;
        };
        if values.is_empty() {
            continue;
        }

        if values.len() == 1 {
            clauses.push(format!("{} = {}", dimension.as_str(), values[0]));
        } else {
            clauses.push(format!("{} in [{}]", dimension.as_str(), values.join(", ")));
        }
    }

    clauses.join(" and ")
}

fn filter_candidates(packets: &[PacketSummary], dimension: FilterDimension) -> Vec<String> {
    let mut candidates = BTreeSet::new();

    for packet in packets {
        match dimension {
            FilterDimension::Host => {
                candidates.insert(endpoint_host(&packet.source));
                candidates.insert(endpoint_host(&packet.destination));
            }
            FilterDimension::Source => {
                candidates.insert(endpoint_host(&packet.source));
            }
            FilterDimension::Destination => {
                candidates.insert(endpoint_host(&packet.destination));
            }
            FilterDimension::Port => {
                if let Some(port) = endpoint_port(&packet.source) {
                    candidates.insert(port.to_string());
                }
                if let Some(port) = endpoint_port(&packet.destination) {
                    candidates.insert(port.to_string());
                }
            }
            FilterDimension::Protocol => {
                candidates.insert(packet.protocol.to_ascii_lowercase());
            }
        }
    }

    candidates.into_iter().collect()
}

fn packet_matches_any_value(
    packet: &PacketSummary,
    dimension: FilterDimension,
    values: &[String],
) -> bool {
    values
        .iter()
        .any(|value| packet_matches_value(packet, dimension, value))
}

fn packet_matches_all_active_filters(app: &App, packet: &PacketSummary) -> bool {
    app.filter_dimensions
        .iter()
        .enumerate()
        .all(|(index, dimension)| {
            let values = app
                .active_filter_values_by_dimension
                .get(index)
                .map(|value| value.as_slice())
                .unwrap_or(&[]);

            values.is_empty() || packet_matches_any_value(packet, *dimension, values)
        })
}

fn packet_matches_value(packet: &PacketSummary, dimension: FilterDimension, value: &str) -> bool {
    let query = value.trim().to_ascii_lowercase();

    match dimension {
        FilterDimension::Host => {
            endpoint_host(&packet.source) == query || endpoint_host(&packet.destination) == query
        }
        FilterDimension::Source => endpoint_host(&packet.source) == query,
        FilterDimension::Destination => endpoint_host(&packet.destination) == query,
        FilterDimension::Port => {
            endpoint_port(&packet.source).is_some_and(|port| port == query)
                || endpoint_port(&packet.destination).is_some_and(|port| port == query)
        }
        FilterDimension::Protocol => packet.protocol.to_ascii_lowercase() == query,
    }
}

fn endpoint_host(endpoint: &str) -> String {
    if let Some((host, port)) = endpoint.rsplit_once('.') {
        if !host.is_empty() && port.chars().all(|ch| ch.is_ascii_digit()) {
            return host.to_ascii_lowercase();
        }
    }
    endpoint.to_ascii_lowercase()
}

fn endpoint_port(endpoint: &str) -> Option<String> {
    let (host, port) = endpoint.rsplit_once('.')?;
    if !host.is_empty() && port.chars().all(|ch| ch.is_ascii_digit()) {
        return Some(port.to_string());
    }
    None
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::{FilterDimension, PacketSummary};

    use super::{App, FocusPane};

    fn sample_packets() -> Vec<PacketSummary> {
        vec![
            PacketSummary {
                timestamp: "1970-01-01 00:00:01.001000".to_string(),
                source: "10.0.0.12.51544".to_string(),
                destination: "1.1.1.1.443".to_string(),
                protocol: "TCP".to_string(),
                length: 0,
                summary: "Flags [S], length 0".to_string(),
            },
            PacketSummary {
                timestamp: "1970-01-01 00:00:02.002000".to_string(),
                source: "10.0.0.12.34211".to_string(),
                destination: "8.8.8.8.53".to_string(),
                protocol: "UDP".to_string(),
                length: 0,
                summary: "UDP, length 0".to_string(),
            },
            PacketSummary {
                timestamp: "1970-01-01 00:00:03.003000".to_string(),
                source: "192.168.1.5.60000".to_string(),
                destination: "1.1.1.1.443".to_string(),
                protocol: "TCP".to_string(),
                length: 0,
                summary: "Flags [.], length 0".to_string(),
            },
        ]
    }

    #[test]
    fn cycle_focus_wraps_back_to_filter_selector() {
        let mut app = App::new();
        assert_eq!(app.focus(), FocusPane::FilterSelector);

        app.cycle_focus();
        assert_eq!(app.focus(), FocusPane::PacketList);

        app.cycle_focus();
        assert_eq!(app.focus(), FocusPane::PacketDetail);

        app.cycle_focus();
        assert_eq!(app.focus(), FocusPane::FilterSelector);
    }

    #[test]
    fn next_packet_stops_at_last_row() {
        let mut app = App::with_packets(sample_packets(), String::new());
        let last_index = app.packets().len() - 1;

        for _ in 0..(app.packets().len() + 3) {
            app.next_packet();
        }

        assert_eq!(app.selected_packet_index(), last_index);
    }

    #[test]
    fn previous_packet_stops_at_zero() {
        let mut app = App::with_packets(sample_packets(), String::new());
        app.previous_packet();

        assert_eq!(app.selected_packet_index(), 0);
    }

    #[test]
    fn selected_filter_dimension_defaults_to_host() {
        let app = App::new();

        assert_eq!(app.selected_filter_dimension(), FilterDimension::Host);
        assert_eq!(app.selected_filter_dimension_index(), 0);
    }

    #[test]
    fn next_filter_dimension_stops_at_last_option() {
        let mut app = App::new();
        let last_index = app.filter_dimensions().len() - 1;

        for _ in 0..(app.filter_dimensions().len() + 3) {
            app.next_filter_dimension();
        }

        assert_eq!(app.selected_filter_dimension_index(), last_index);
        assert_eq!(app.selected_filter_dimension(), FilterDimension::Protocol);
    }

    #[test]
    fn previous_filter_dimension_stops_at_zero() {
        let mut app = App::new();
        app.previous_filter_dimension();

        assert_eq!(app.selected_filter_dimension_index(), 0);
        assert_eq!(app.selected_filter_dimension(), FilterDimension::Host);
    }

    #[test]
    fn move_down_in_packet_list_focus_advances_packet_selection() {
        let mut app = App::with_packets(sample_packets(), String::new());
        app.cycle_focus();
        assert_eq!(app.focus(), FocusPane::PacketList);

        app.move_down();

        assert_eq!(app.selected_packet_index(), 1);
    }

    #[test]
    fn move_down_in_filter_selector_focus_advances_filter_selection() {
        let mut app = App::new();
        assert_eq!(app.focus(), FocusPane::FilterSelector);

        app.move_down();

        assert_eq!(app.selected_filter_dimension(), FilterDimension::Source);
    }

    #[test]
    fn reverse_cycle_focus_wraps_back_to_filter_selector() {
        let mut app = App::new();
        assert_eq!(app.focus(), FocusPane::FilterSelector);

        app.reverse_cycle_focus();
        assert_eq!(app.focus(), FocusPane::PacketDetail);

        app.reverse_cycle_focus();
        assert_eq!(app.focus(), FocusPane::PacketList);

        app.reverse_cycle_focus();
        assert_eq!(app.focus(), FocusPane::FilterSelector);
    }

    #[test]
    fn opening_host_popup_lists_unique_hosts() {
        let mut app = App::with_packets(sample_packets(), String::new());

        app.open_filter_popup();

        assert!(app.is_filter_popup_open());
        assert_eq!(app.filter_popup_dimension(), Some(FilterDimension::Host));
        let candidates = app
            .filter_popup_candidates()
            .expect("host popup should expose candidates");
        assert_eq!(
            candidates,
            &[
                "1.1.1.1".to_string(),
                "10.0.0.12".to_string(),
                "192.168.1.5".to_string(),
                "8.8.8.8".to_string(),
            ]
        );
    }

    #[test]
    fn popup_selection_with_space_and_enter_applies_filter_expression() {
        let mut app = App::with_packets(sample_packets(), String::new());

        app.open_filter_popup();
        app.move_down();
        app.move_down();
        app.move_down();
        assert_eq!(app.filter_popup_selected_index(), Some(3));

        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();

        assert!(!app.is_filter_popup_open());
        assert_eq!(app.filter_expression(), "host = 8.8.8.8");
        assert_eq!(app.packets().len(), 1);
        assert_eq!(app.packets()[0].destination, "8.8.8.8.53");
    }

    #[test]
    fn popup_move_down_wraps_from_last_item_to_first() {
        let mut app = App::with_packets(sample_packets(), String::new());

        app.open_filter_popup();
        app.move_down();
        app.move_down();
        app.move_down();
        assert_eq!(app.filter_popup_selected_index(), Some(3));

        app.move_down();
        assert_eq!(app.filter_popup_selected_index(), Some(0));
    }

    #[test]
    fn popup_move_up_wraps_from_first_item_to_last() {
        let mut app = App::with_packets(sample_packets(), String::new());

        app.open_filter_popup();
        assert_eq!(app.filter_popup_selected_index(), Some(0));

        app.move_up();
        assert_eq!(app.filter_popup_selected_index(), Some(3));
    }

    #[test]
    fn popup_supports_multi_select_and_expression_lists_selected_values() {
        let mut app = App::with_packets(sample_packets(), String::new());

        for _ in 0..4 {
            app.next_filter_dimension();
        }
        assert_eq!(app.selected_filter_dimension(), FilterDimension::Protocol);

        app.open_filter_popup();
        let candidates = app
            .filter_popup_candidates()
            .expect("protocol popup should expose candidates");
        assert_eq!(candidates, &["tcp".to_string(), "udp".to_string()]);

        app.toggle_filter_popup_selection();
        app.move_down();
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();

        assert_eq!(app.filter_expression(), "protocol in [tcp, udp]");
        assert_eq!(app.packets().len(), 3);
    }

    #[test]
    fn popup_confirm_without_selections_clears_active_filter() {
        let mut app = App::with_packets(sample_packets(), String::new());

        for _ in 0..4 {
            app.next_filter_dimension();
        }

        app.open_filter_popup();
        app.move_down();
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();

        assert_eq!(app.filter_expression(), "protocol = udp");
        assert_eq!(app.packets().len(), 1);

        app.open_filter_popup();
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();

        assert_eq!(app.filter_expression(), "");
        assert_eq!(app.packets().len(), 3);
    }

    #[test]
    fn changing_filter_dimension_preserves_applied_values() {
        let mut app = App::with_packets(sample_packets(), String::new());

        app.open_filter_popup();
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();
        assert_eq!(app.filter_expression(), "host = 1.1.1.1");
        assert_eq!(app.packets().len(), 2);

        app.next_filter_dimension();

        assert_eq!(app.selected_filter_dimension(), FilterDimension::Source);
        assert_eq!(app.filter_expression(), "host = 1.1.1.1");
        assert_eq!(app.packets().len(), 2);
    }

    #[test]
    fn filters_across_dimensions_are_combined_with_and() {
        let mut app = App::with_packets(sample_packets(), String::new());

        app.open_filter_popup();
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();
        assert_eq!(app.filter_expression(), "host = 1.1.1.1");
        assert_eq!(app.packets().len(), 2);

        for _ in 0..4 {
            app.next_filter_dimension();
        }
        assert_eq!(app.selected_filter_dimension(), FilterDimension::Protocol);

        app.open_filter_popup();
        app.move_down();
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();

        assert_eq!(app.filter_expression(), "host = 1.1.1.1 and protocol = udp");
        assert_eq!(app.packets().len(), 0);
    }
}

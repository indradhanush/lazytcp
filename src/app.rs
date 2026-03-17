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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateTimePopupField {
    Start,
    End,
}

#[derive(Debug, Clone)]
enum FilterPopupState {
    MultiSelect {
        all_candidates: Vec<String>,
        candidates: Vec<String>,
        selected_values: BTreeSet<String>,
        highlighted_index: usize,
        search_query: String,
        search_active: bool,
    },
    DateTimeRange {
        start_input: String,
        end_input: String,
        active_field: DateTimePopupField,
    },
}

#[derive(Debug, Clone)]
struct FilterPopup {
    dimension: FilterDimension,
    state: FilterPopupState,
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
    keybindings_popup_open: bool,
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
            keybindings_popup_open: false,
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

    pub fn is_filter_dimension_active(&self, dimension: FilterDimension) -> bool {
        let Some(index) = self
            .filter_dimensions
            .iter()
            .position(|candidate| *candidate == dimension)
        else {
            return false;
        };

        self.active_filter_values_by_dimension
            .get(index)
            .is_some_and(|values| !values.is_empty())
    }

    pub fn is_filter_popup_open(&self) -> bool {
        self.filter_popup.is_some()
    }

    pub fn is_keybindings_popup_open(&self) -> bool {
        self.keybindings_popup_open
    }

    pub fn open_keybindings_popup(&mut self) {
        self.keybindings_popup_open = true;
    }

    pub fn close_keybindings_popup(&mut self) {
        self.keybindings_popup_open = false;
    }

    pub fn filter_popup_dimension(&self) -> Option<FilterDimension> {
        self.filter_popup.as_ref().map(|popup| popup.dimension)
    }

    pub fn is_filter_popup_date_time(&self) -> bool {
        self.filter_popup
            .as_ref()
            .is_some_and(|popup| popup.dimension == FilterDimension::DateTime)
    }

    pub fn is_filter_popup_search_active(&self) -> bool {
        let Some(popup) = self.filter_popup.as_ref() else {
            return false;
        };

        let FilterPopupState::MultiSelect { search_active, .. } = &popup.state else {
            return false;
        };

        *search_active
    }

    pub fn filter_popup_search_query(&self) -> Option<&str> {
        let popup = self.filter_popup.as_ref()?;
        let FilterPopupState::MultiSelect { search_query, .. } = &popup.state else {
            return None;
        };
        Some(search_query.as_str())
    }

    pub fn filter_popup_candidates(&self) -> Option<&[String]> {
        let popup = self.filter_popup.as_ref()?;
        match &popup.state {
            FilterPopupState::MultiSelect { candidates, .. } => Some(candidates.as_slice()),
            FilterPopupState::DateTimeRange { .. } => None,
        }
    }

    pub fn filter_popup_selected_index(&self) -> Option<usize> {
        let popup = self.filter_popup.as_ref()?;
        match popup.state {
            FilterPopupState::MultiSelect {
                highlighted_index, ..
            } => Some(highlighted_index),
            FilterPopupState::DateTimeRange { .. } => None,
        }
    }

    pub fn filter_popup_candidate_selected(&self, index: usize) -> bool {
        let Some(popup) = self.filter_popup.as_ref() else {
            return false;
        };

        let FilterPopupState::MultiSelect {
            candidates,
            selected_values,
            ..
        } = &popup.state
        else {
            return false;
        };

        let Some(candidate) = candidates.get(index) else {
            return false;
        };

        selected_values.contains(candidate)
    }

    pub fn filter_popup_date_time_start_input(&self) -> Option<&str> {
        let popup = self.filter_popup.as_ref()?;
        let FilterPopupState::DateTimeRange { start_input, .. } = &popup.state else {
            return None;
        };
        Some(start_input.as_str())
    }

    pub fn filter_popup_date_time_end_input(&self) -> Option<&str> {
        let popup = self.filter_popup.as_ref()?;
        let FilterPopupState::DateTimeRange { end_input, .. } = &popup.state else {
            return None;
        };
        Some(end_input.as_str())
    }

    pub fn filter_popup_date_time_active_field(&self) -> Option<DateTimePopupField> {
        let popup = self.filter_popup.as_ref()?;
        let FilterPopupState::DateTimeRange { active_field, .. } = popup.state else {
            return None;
        };
        Some(active_field)
    }

    pub fn open_filter_popup(&mut self) {
        let dimension = self.selected_filter_dimension();
        self.filter_popup = Some(match dimension {
            FilterDimension::DateTime => {
                let range = decode_date_time_range_filter_values(
                    self.active_filter_values_for_selected_dimension(),
                );
                FilterPopup {
                    dimension,
                    state: FilterPopupState::DateTimeRange {
                        start_input: range.start.unwrap_or_default(),
                        end_input: range.end.unwrap_or_default(),
                        active_field: DateTimePopupField::Start,
                    },
                }
            }
            _ => {
                let all_candidates = filter_candidates(
                    self.all_packets.iter().filter(|packet| {
                        packet_matches_all_active_filters_except_dimension(
                            self,
                            packet,
                            Some(dimension),
                        )
                    }),
                    dimension,
                );
                let selected_values: BTreeSet<String> = self
                    .active_filter_values_for_selected_dimension()
                    .iter()
                    .cloned()
                    .collect();
                let highlighted_index = all_candidates
                    .iter()
                    .position(|candidate| selected_values.contains(candidate))
                    .unwrap_or(0);

                FilterPopup {
                    dimension,
                    state: FilterPopupState::MultiSelect {
                        candidates: all_candidates.clone(),
                        all_candidates,
                        selected_values,
                        highlighted_index,
                        search_query: String::new(),
                        search_active: false,
                    },
                }
            }
        });
    }

    pub fn close_filter_popup(&mut self) {
        self.filter_popup = None;
    }

    pub fn confirm_filter_popup(&mut self) {
        let Some(popup) = self.filter_popup.take() else {
            return;
        };

        let (selected_values, focus_after_confirm) = match popup.state {
            FilterPopupState::MultiSelect {
                all_candidates,
                selected_values,
                ..
            } => (
                all_candidates
                    .iter()
                    .filter(|candidate| selected_values.contains(*candidate))
                    .cloned()
                    .collect(),
                FocusPane::FilterSelector,
            ),
            FilterPopupState::DateTimeRange {
                start_input,
                end_input,
                ..
            } => (
                encode_date_time_range_filter_values(
                    normalize_date_time_popup_input(&start_input),
                    normalize_date_time_popup_input(&end_input),
                ),
                FocusPane::PacketList,
            ),
        };
        self.set_active_filter_values(popup.dimension, selected_values);
        self.filter_expression = build_filter_expression(
            &self.filter_dimensions,
            &self.active_filter_values_by_dimension,
        );
        self.apply_active_filter();
        self.focus = focus_after_confirm;
    }

    pub fn toggle_filter_popup_selection(&mut self) {
        let Some(popup) = self.filter_popup.as_mut() else {
            return;
        };

        let FilterPopupState::MultiSelect {
            candidates,
            selected_values,
            highlighted_index,
            ..
        } = &mut popup.state
        else {
            return;
        };

        let Some(candidate) = candidates.get(*highlighted_index).cloned() else {
            return;
        };

        if !selected_values.insert(candidate.clone()) {
            selected_values.remove(&candidate);
        }
    }

    pub fn clear_filter_popup_selection(&mut self) {
        let Some(popup) = self.filter_popup.as_mut() else {
            return;
        };

        match &mut popup.state {
            FilterPopupState::MultiSelect {
                selected_values, ..
            } => selected_values.clear(),
            FilterPopupState::DateTimeRange {
                start_input,
                end_input,
                ..
            } => {
                start_input.clear();
                end_input.clear();
            }
        }
    }

    pub fn filter_popup_switch_date_time_field(&mut self) {
        let Some(popup) = self.filter_popup.as_mut() else {
            return;
        };
        let FilterPopupState::DateTimeRange { active_field, .. } = &mut popup.state else {
            return;
        };
        *active_field = match active_field {
            DateTimePopupField::Start => DateTimePopupField::End,
            DateTimePopupField::End => DateTimePopupField::Start,
        };
    }

    pub fn start_filter_popup_search(&mut self) {
        let Some(popup) = self.filter_popup.as_mut() else {
            return;
        };
        let FilterPopupState::MultiSelect {
            all_candidates,
            candidates,
            selected_values,
            highlighted_index,
            search_query,
            search_active,
        } = &mut popup.state
        else {
            return;
        };

        *search_active = true;
        search_query.clear();
        refresh_popup_search_candidates(
            all_candidates,
            candidates,
            selected_values,
            highlighted_index,
            search_query,
        );
    }

    pub fn filter_popup_search_insert_char(&mut self, ch: char) {
        if ch.is_control() {
            return;
        }

        let Some(popup) = self.filter_popup.as_mut() else {
            return;
        };
        let FilterPopupState::MultiSelect {
            all_candidates,
            candidates,
            selected_values,
            highlighted_index,
            search_query,
            search_active,
        } = &mut popup.state
        else {
            return;
        };
        if !*search_active {
            return;
        }

        search_query.push(ch);
        refresh_popup_search_candidates(
            all_candidates,
            candidates,
            selected_values,
            highlighted_index,
            search_query,
        );
    }

    pub fn filter_popup_search_backspace(&mut self) {
        let Some(popup) = self.filter_popup.as_mut() else {
            return;
        };
        let FilterPopupState::MultiSelect {
            all_candidates,
            candidates,
            selected_values,
            highlighted_index,
            search_query,
            search_active,
        } = &mut popup.state
        else {
            return;
        };
        if !*search_active {
            return;
        }

        search_query.pop();
        refresh_popup_search_candidates(
            all_candidates,
            candidates,
            selected_values,
            highlighted_index,
            search_query,
        );
    }

    pub fn stop_filter_popup_search(&mut self) {
        let Some(popup) = self.filter_popup.as_mut() else {
            return;
        };
        let FilterPopupState::MultiSelect { search_active, .. } = &mut popup.state else {
            return;
        };
        *search_active = false;
    }

    pub fn filter_popup_insert_char(&mut self, ch: char) {
        let Some(popup) = self.filter_popup.as_mut() else {
            return;
        };
        let FilterPopupState::DateTimeRange {
            start_input,
            end_input,
            active_field,
        } = &mut popup.state
        else {
            return;
        };

        if !is_allowed_date_time_char(ch) {
            return;
        }

        match active_field {
            DateTimePopupField::Start => start_input.push(ch),
            DateTimePopupField::End => end_input.push(ch),
        }
    }

    pub fn filter_popup_backspace(&mut self) {
        let Some(popup) = self.filter_popup.as_mut() else {
            return;
        };
        let FilterPopupState::DateTimeRange {
            start_input,
            end_input,
            active_field,
        } = &mut popup.state
        else {
            return;
        };

        match active_field {
            DateTimePopupField::Start => {
                start_input.pop();
            }
            DateTimePopupField::End => {
                end_input.pop();
            }
        }
    }

    pub fn clear_all_filters(&mut self) {
        for values in &mut self.active_filter_values_by_dimension {
            values.clear();
        }
        self.filter_expression.clear();

        if let Some(popup) = self.filter_popup.as_mut() {
            match &mut popup.state {
                FilterPopupState::MultiSelect {
                    selected_values, ..
                } => selected_values.clear(),
                FilterPopupState::DateTimeRange {
                    start_input,
                    end_input,
                    ..
                } => {
                    start_input.clear();
                    end_input.clear();
                }
            }
        }

        self.apply_active_filter();
    }

    pub fn clear_selected_filter_dimension(&mut self) {
        if let Some(values) = self
            .active_filter_values_by_dimension
            .get_mut(self.selected_filter_dimension)
        {
            values.clear();
        }

        if let Some(popup) = self.filter_popup.as_mut() {
            match &mut popup.state {
                FilterPopupState::MultiSelect {
                    selected_values, ..
                } => selected_values.clear(),
                FilterPopupState::DateTimeRange {
                    start_input,
                    end_input,
                    ..
                } => {
                    start_input.clear();
                    end_input.clear();
                }
            }
        }

        self.filter_expression = build_filter_expression(
            &self.filter_dimensions,
            &self.active_filter_values_by_dimension,
        );
        self.apply_active_filter();
    }

    pub fn focus_filter_input(&mut self) {
        self.focus = FocusPane::FilterInput;
    }

    pub fn focus_filter_selector(&mut self) {
        self.focus = FocusPane::FilterSelector;
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
            match &mut popup.state {
                FilterPopupState::MultiSelect {
                    candidates,
                    highlighted_index,
                    ..
                } => {
                    if candidates.is_empty() {
                        return;
                    }
                    *highlighted_index = (*highlighted_index + 1) % candidates.len();
                }
                FilterPopupState::DateTimeRange { active_field, .. } => {
                    *active_field = DateTimePopupField::End;
                }
            }
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
            match &mut popup.state {
                FilterPopupState::MultiSelect {
                    candidates,
                    highlighted_index,
                    ..
                } => {
                    if candidates.is_empty() {
                        return;
                    }
                    *highlighted_index = if *highlighted_index == 0 {
                        candidates.len() - 1
                    } else {
                        *highlighted_index - 1
                    };
                }
                FilterPopupState::DateTimeRange { active_field, .. } => {
                    *active_field = DateTimePopupField::Start;
                }
            }
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

        if *dimension == FilterDimension::DateTime {
            if let Some(clause) = build_date_time_filter_clause(values) {
                clauses.push(clause);
            }
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

#[derive(Debug, Default, Clone)]
struct DateTimeRangeFilterValues {
    start: Option<String>,
    end: Option<String>,
}

fn normalize_date_time_popup_input(value: &str) -> Option<String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized.to_string())
    }
}

fn encode_date_time_range_filter_values(start: Option<String>, end: Option<String>) -> Vec<String> {
    let mut values = Vec::new();
    if let Some(start) = start {
        values.push(format!("start={start}"));
    }
    if let Some(end) = end {
        values.push(format!("end={end}"));
    }
    values
}

fn decode_date_time_range_filter_values(values: &[String]) -> DateTimeRangeFilterValues {
    let mut result = DateTimeRangeFilterValues::default();

    for value in values {
        if let Some(start) = value.strip_prefix("start=") {
            result.start = normalize_date_time_popup_input(start);
            continue;
        }
        if let Some(end) = value.strip_prefix("end=") {
            result.end = normalize_date_time_popup_input(end);
        }
    }

    result
}

fn build_date_time_filter_clause(values: &[String]) -> Option<String> {
    let range = decode_date_time_range_filter_values(values);
    match (range.start.as_deref(), range.end.as_deref()) {
        (None, None) => None,
        (Some(start), None) => Some(format!("date time >= {start}")),
        (None, Some(end)) => Some(format!("date time <= {end}")),
        (Some(start), Some(end)) => {
            let (start, end) = if start <= end {
                (start, end)
            } else {
                (end, start)
            };
            Some(format!("date time between [{start} .. {end}]"))
        }
    }
}

fn packet_matches_date_time_filter(packet: &PacketSummary, values: &[String]) -> bool {
    let range = decode_date_time_range_filter_values(values);
    match (range.start.as_deref(), range.end.as_deref()) {
        (None, None) => true,
        (Some(start), None) => packet.timestamp.as_str() >= start,
        (None, Some(end)) => packet.timestamp.as_str() <= end,
        (Some(start), Some(end)) => {
            let (start, end) = if start <= end {
                (start, end)
            } else {
                (end, start)
            };
            packet.timestamp.as_str() >= start && packet.timestamp.as_str() <= end
        }
    }
}

fn is_allowed_date_time_char(ch: char) -> bool {
    ch.is_ascii_digit() || matches!(ch, '-' | ':' | '.' | ' ')
}

fn refresh_popup_search_candidates(
    all_candidates: &[String],
    candidates: &mut Vec<String>,
    selected_values: &BTreeSet<String>,
    highlighted_index: &mut usize,
    search_query: &str,
) {
    let previously_highlighted = candidates.get(*highlighted_index).cloned();
    let normalized_query = search_query.to_ascii_lowercase();

    *candidates = if normalized_query.is_empty() {
        all_candidates.to_vec()
    } else {
        all_candidates
            .iter()
            .filter(|candidate| candidate.to_ascii_lowercase().contains(&normalized_query))
            .cloned()
            .collect()
    };

    if candidates.is_empty() {
        *highlighted_index = 0;
        return;
    }

    if let Some(highlighted) = previously_highlighted {
        if let Some(index) = candidates
            .iter()
            .position(|candidate| candidate == &highlighted)
        {
            *highlighted_index = index;
            return;
        }
    }

    if let Some(index) = candidates
        .iter()
        .position(|candidate| selected_values.contains(candidate))
    {
        *highlighted_index = index;
        return;
    }

    *highlighted_index = 0;
}

fn filter_candidates<'a>(
    packets: impl IntoIterator<Item = &'a PacketSummary>,
    dimension: FilterDimension,
) -> Vec<String> {
    if dimension == FilterDimension::TcpFlags {
        return all_tcp_flag_labels()
            .iter()
            .map(|label| (*label).to_string())
            .collect();
    }

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
            FilterDimension::Interface => {
                if let Some(interface) = packet.interface.as_deref() {
                    candidates.insert(interface.to_ascii_lowercase());
                }
            }
            FilterDimension::Port => {
                if let Some(port) = endpoint_port(&packet.source) {
                    candidates.insert(port.to_string());
                }
                if let Some(port) = endpoint_port(&packet.destination) {
                    candidates.insert(port.to_string());
                }
            }
            FilterDimension::SourcePort => {
                if let Some(port) = endpoint_port(&packet.source) {
                    candidates.insert(port.to_string());
                }
            }
            FilterDimension::DestinationPort => {
                if let Some(port) = endpoint_port(&packet.destination) {
                    candidates.insert(port.to_string());
                }
            }
            FilterDimension::Protocol => {
                candidates.insert(packet.protocol.to_ascii_lowercase());
            }
            FilterDimension::TcpFlags => {}
            FilterDimension::IpVersion => {
                if let Some(version) = packet_ip_version(packet) {
                    candidates.insert(version.to_string());
                }
            }
            FilterDimension::DateTime => {
                candidates.insert(packet.timestamp.clone());
            }
            FilterDimension::TrafficClass => {
                if let Some(class) = packet_traffic_class(packet) {
                    candidates.insert(class.to_string());
                }
            }
            FilterDimension::IcmpType => {
                if let Some(icmp_type) = packet_icmp_type(packet) {
                    candidates.insert(icmp_type);
                }
            }
        }
    }

    let mut candidates: Vec<String> = candidates.into_iter().collect();
    if matches!(
        dimension,
        FilterDimension::Port | FilterDimension::SourcePort | FilterDimension::DestinationPort
    ) {
        candidates.sort_by(|left, right| {
            let left_port = left.parse::<u16>().ok();
            let right_port = right.parse::<u16>().ok();
            left_port.cmp(&right_port).then_with(|| left.cmp(right))
        });
    }

    candidates
}

fn packet_matches_any_value(
    packet: &PacketSummary,
    dimension: FilterDimension,
    values: &[String],
) -> bool {
    if dimension == FilterDimension::TcpFlags {
        return packet_matches_exact_tcp_flag_set(packet, values.iter().map(String::as_str));
    }

    values
        .iter()
        .any(|value| packet_matches_value(packet, dimension, value))
}

fn packet_matches_exact_tcp_flag_set<'a>(
    packet: &PacketSummary,
    selected_values: impl IntoIterator<Item = &'a str>,
) -> bool {
    let selected_flags: BTreeSet<String> = selected_values
        .into_iter()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();

    if selected_flags.is_empty() {
        return false;
    }

    let packet_flags: BTreeSet<&'static str> = tcp_flags_from_summary(&packet.summary)
        .into_iter()
        .collect();

    packet_flags.len() == selected_flags.len()
        && selected_flags
            .iter()
            .all(|selected_flag| packet_flags.contains(selected_flag.as_str()))
}

fn packet_matches_all_active_filters(app: &App, packet: &PacketSummary) -> bool {
    packet_matches_all_active_filters_except_dimension(app, packet, None)
}

fn packet_matches_all_active_filters_except_dimension(
    app: &App,
    packet: &PacketSummary,
    excluded_dimension: Option<FilterDimension>,
) -> bool {
    app.filter_dimensions
        .iter()
        .enumerate()
        .all(|(index, dimension)| {
            if excluded_dimension.is_some_and(|excluded| *dimension == excluded) {
                return true;
            }

            let values = app
                .active_filter_values_by_dimension
                .get(index)
                .map(|value| value.as_slice())
                .unwrap_or(&[]);

            if values.is_empty() {
                return true;
            }
            if *dimension == FilterDimension::DateTime {
                return packet_matches_date_time_filter(packet, values);
            }

            packet_matches_any_value(packet, *dimension, values)
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
        FilterDimension::Interface => packet
            .interface
            .as_deref()
            .is_some_and(|interface| interface.eq_ignore_ascii_case(&query)),
        FilterDimension::Port => {
            endpoint_port(&packet.source).is_some_and(|port| port == query)
                || endpoint_port(&packet.destination).is_some_and(|port| port == query)
        }
        FilterDimension::SourcePort => {
            endpoint_port(&packet.source).is_some_and(|port| port == query)
        }
        FilterDimension::DestinationPort => {
            endpoint_port(&packet.destination).is_some_and(|port| port == query)
        }
        FilterDimension::Protocol => packet.protocol.to_ascii_lowercase() == query,
        FilterDimension::TcpFlags => {
            packet_matches_exact_tcp_flag_set(packet, std::iter::once(value))
        }
        FilterDimension::IpVersion => {
            packet_ip_version(packet).is_some_and(|version| version == query)
        }
        FilterDimension::DateTime => packet.timestamp == value.trim(),
        FilterDimension::TrafficClass => {
            packet_traffic_class(packet).is_some_and(|class| class == query)
        }
        FilterDimension::IcmpType => packet_icmp_type(packet)
            .as_deref()
            .is_some_and(|icmp_type| icmp_type == query),
    }
}

fn tcp_flags_from_summary(summary: &str) -> Vec<&'static str> {
    let Some(start) = summary.find("Flags [") else {
        return Vec::new();
    };
    let remainder = &summary[start + "Flags [".len()..];
    let Some(end) = remainder.find(']') else {
        return Vec::new();
    };

    let mut flags = Vec::new();
    for symbol in remainder[..end].chars() {
        let Some(label) = tcp_flag_label(symbol) else {
            continue;
        };
        flags.push(label);
    }

    flags
}

fn all_tcp_flag_labels() -> [&'static str; 9] {
    ["NS", "CWR", "ECE", "URG", "ACK", "PSH", "RST", "SYN", "FIN"]
}

fn tcp_flag_label(symbol: char) -> Option<&'static str> {
    match symbol {
        'N' => Some("ns"),
        'W' => Some("cwr"),
        'E' => Some("ece"),
        'U' => Some("urg"),
        '.' => Some("ack"),
        'P' => Some("psh"),
        'R' => Some("rst"),
        'S' => Some("syn"),
        'F' => Some("fin"),
        _ => None,
    }
}

fn endpoint_host(endpoint: &str) -> String {
    split_endpoint_host_port(endpoint)
        .map(|(host, _)| host.to_ascii_lowercase())
        .unwrap_or_else(|| endpoint.to_ascii_lowercase())
}

fn packet_ip_version(packet: &PacketSummary) -> Option<&'static str> {
    endpoint_ip_version(&packet.source).or_else(|| endpoint_ip_version(&packet.destination))
}

fn endpoint_ip_version(endpoint: &str) -> Option<&'static str> {
    let host = split_endpoint_host_port(endpoint)
        .map(|(host, _)| host)
        .unwrap_or(endpoint);

    if host.contains(':') {
        return Some("ipv6");
    }
    if is_ipv4_address(host) {
        return Some("ipv4");
    }
    None
}

fn packet_traffic_class(packet: &PacketSummary) -> Option<&'static str> {
    endpoint_traffic_class(&packet.destination)
}

fn endpoint_traffic_class(endpoint: &str) -> Option<&'static str> {
    let host = split_endpoint_host_port(endpoint)
        .map(|(host, _)| host)
        .unwrap_or(endpoint)
        .to_ascii_lowercase();

    if host.contains(':') {
        return Some(if host.starts_with("ff") {
            "multicast"
        } else {
            "unicast"
        });
    }

    if !is_ipv4_address(&host) {
        return None;
    }

    let octets: Vec<u8> = host
        .split('.')
        .filter_map(|part| part.parse::<u8>().ok())
        .collect();
    if octets.len() != 4 {
        return None;
    }

    if host == "255.255.255.255" || octets[3] == 255 {
        return Some("broadcast");
    }
    if (224..=239).contains(&octets[0]) {
        return Some("multicast");
    }

    Some("unicast")
}

fn packet_icmp_type(packet: &PacketSummary) -> Option<String> {
    if !packet.protocol.eq_ignore_ascii_case("ICMP")
        && !packet.protocol.eq_ignore_ascii_case("ICMP6")
    {
        return None;
    }

    icmp_type_from_summary(&packet.summary)
}

fn icmp_type_from_summary(summary: &str) -> Option<String> {
    let trimmed = summary.trim();
    let remainder = if let Some(rest) = trimmed.strip_prefix("ICMP6") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("ICMP") {
        rest
    } else {
        return None;
    };

    let normalized = remainder.trim_start_matches([',', ':', ' ']).trim_start();
    if normalized.is_empty() {
        return None;
    }

    let type_label = normalized
        .split(',')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(type_label.to_ascii_lowercase())
}

fn endpoint_port(endpoint: &str) -> Option<String> {
    split_endpoint_host_port(endpoint).map(|(_, port)| port.to_string())
}

fn split_endpoint_host_port(endpoint: &str) -> Option<(&str, &str)> {
    let (host, port) = endpoint.rsplit_once('.')?;
    if host.is_empty() || port.is_empty() || !port.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }

    if host.contains(':') || is_ipv4_address(host) {
        return Some((host, port));
    }

    None
}

fn is_ipv4_address(value: &str) -> bool {
    let mut parts = value.split('.');
    let mut count = 0;

    for part in parts.by_ref() {
        count += 1;
        if part.is_empty() || !part.chars().all(|ch| ch.is_ascii_digit()) {
            return false;
        }
        if part.parse::<u8>().is_err() {
            return false;
        }
    }

    count == 4
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::{FilterDimension, PacketSummary};

    use super::{endpoint_host, endpoint_port, App, DateTimePopupField, FocusPane};

    fn sample_packets() -> Vec<PacketSummary> {
        vec![
            PacketSummary {
                timestamp: "1970-01-01 00:00:01.001000".to_string(),
                interface: None,
                source: "10.0.0.12.51544".to_string(),
                destination: "1.1.1.1.443".to_string(),
                protocol: "TCP".to_string(),
                length: 0,
                summary: "Flags [S], length 0".to_string(),
            },
            PacketSummary {
                timestamp: "1970-01-01 00:00:02.002000".to_string(),
                interface: None,
                source: "10.0.0.12.34211".to_string(),
                destination: "8.8.8.8.53".to_string(),
                protocol: "UDP".to_string(),
                length: 0,
                summary: "UDP, length 0".to_string(),
            },
            PacketSummary {
                timestamp: "1970-01-01 00:00:03.003000".to_string(),
                interface: None,
                source: "192.168.1.5.60000".to_string(),
                destination: "1.1.1.1.443".to_string(),
                protocol: "TCP".to_string(),
                length: 0,
                summary: "Flags [.], length 0".to_string(),
            },
        ]
    }

    fn sample_packets_with_ipv6() -> Vec<PacketSummary> {
        let mut packets = sample_packets();
        packets.push(PacketSummary {
            timestamp: "1970-01-01 00:00:04.004000".to_string(),
            interface: None,
            source: "fe80::1.5353".to_string(),
            destination: "ff02::fb.5353".to_string(),
            protocol: "UDP".to_string(),
            length: 32,
            summary: "UDP, length 32".to_string(),
        });
        packets
    }

    fn sample_packets_with_traffic_classes() -> Vec<PacketSummary> {
        vec![
            PacketSummary {
                timestamp: "1970-01-01 00:00:01.001000".to_string(),
                interface: None,
                source: "10.0.0.12.51544".to_string(),
                destination: "1.1.1.1.443".to_string(),
                protocol: "TCP".to_string(),
                length: 0,
                summary: "Flags [S], length 0".to_string(),
            },
            PacketSummary {
                timestamp: "1970-01-01 00:00:02.002000".to_string(),
                interface: None,
                source: "192.168.1.10.5353".to_string(),
                destination: "224.0.0.251.5353".to_string(),
                protocol: "UDP".to_string(),
                length: 32,
                summary: "UDP, length 32".to_string(),
            },
            PacketSummary {
                timestamp: "1970-01-01 00:00:03.003000".to_string(),
                interface: None,
                source: "0.0.0.0.68".to_string(),
                destination: "255.255.255.255.67".to_string(),
                protocol: "UDP".to_string(),
                length: 300,
                summary: "UDP, length 300".to_string(),
            },
        ]
    }

    fn sample_packets_with_icmp_types() -> Vec<PacketSummary> {
        vec![
            PacketSummary {
                timestamp: "1970-01-01 00:00:01.001000".to_string(),
                interface: None,
                source: "10.0.0.12".to_string(),
                destination: "1.1.1.1".to_string(),
                protocol: "ICMP".to_string(),
                length: 84,
                summary: "ICMP echo request, id 1, seq 1, length 64".to_string(),
            },
            PacketSummary {
                timestamp: "1970-01-01 00:00:02.002000".to_string(),
                interface: None,
                source: "1.1.1.1".to_string(),
                destination: "10.0.0.12".to_string(),
                protocol: "ICMP".to_string(),
                length: 84,
                summary: "ICMP echo reply, id 1, seq 1, length 64".to_string(),
            },
            PacketSummary {
                timestamp: "1970-01-01 00:00:03.003000".to_string(),
                interface: None,
                source: "fe80::1".to_string(),
                destination: "ff02::1:ff00:1".to_string(),
                protocol: "ICMP6".to_string(),
                length: 72,
                summary: "ICMP6, neighbor solicitation, who has fe80::1, length 32".to_string(),
            },
        ]
    }

    fn sample_packets_with_tcp_flag_combinations() -> Vec<PacketSummary> {
        vec![
            PacketSummary {
                timestamp: "1970-01-01 00:00:01.001000".to_string(),
                interface: None,
                source: "10.0.0.12.51544".to_string(),
                destination: "1.1.1.1.443".to_string(),
                protocol: "TCP".to_string(),
                length: 0,
                summary: "Flags [.], length 0".to_string(),
            },
            PacketSummary {
                timestamp: "1970-01-01 00:00:02.002000".to_string(),
                interface: None,
                source: "10.0.0.12.51545".to_string(),
                destination: "1.1.1.1.443".to_string(),
                protocol: "TCP".to_string(),
                length: 0,
                summary: "Flags [P.], length 0".to_string(),
            },
            PacketSummary {
                timestamp: "1970-01-01 00:00:03.003000".to_string(),
                interface: None,
                source: "10.0.0.12.51546".to_string(),
                destination: "1.1.1.1.443".to_string(),
                protocol: "TCP".to_string(),
                length: 0,
                summary: "Flags [S.], length 0".to_string(),
            },
            PacketSummary {
                timestamp: "1970-01-01 00:00:04.004000".to_string(),
                interface: None,
                source: "10.0.0.12.51547".to_string(),
                destination: "1.1.1.1.443".to_string(),
                protocol: "TCP".to_string(),
                length: 0,
                summary: "Flags [S], length 0".to_string(),
            },
        ]
    }

    fn sample_packets_with_arp() -> Vec<PacketSummary> {
        let mut packets = sample_packets();
        packets.push(PacketSummary {
            timestamp: "1970-01-01 00:00:04.004000".to_string(),
            interface: None,
            source: "10.0.0.2".to_string(),
            destination: "10.0.0.1".to_string(),
            protocol: "ARP".to_string(),
            length: 46,
            summary: "Request who-has 10.0.0.1 tell 10.0.0.2, length 46".to_string(),
        });
        packets
    }

    fn sample_packets_with_interfaces() -> Vec<PacketSummary> {
        vec![
            PacketSummary {
                timestamp: "1970-01-01 00:00:01.001000".to_string(),
                interface: Some("en0".to_string()),
                source: "10.0.0.12.51544".to_string(),
                destination: "1.1.1.1.443".to_string(),
                protocol: "TCP".to_string(),
                length: 0,
                summary: "Flags [S], length 0".to_string(),
            },
            PacketSummary {
                timestamp: "1970-01-01 00:00:02.002000".to_string(),
                interface: Some("utun0".to_string()),
                source: "10.0.0.12.34211".to_string(),
                destination: "8.8.8.8.53".to_string(),
                protocol: "UDP".to_string(),
                length: 0,
                summary: "UDP, length 0".to_string(),
            },
            PacketSummary {
                timestamp: "1970-01-01 00:00:03.003000".to_string(),
                interface: Some("en0".to_string()),
                source: "192.168.1.5.60000".to_string(),
                destination: "1.1.1.1.443".to_string(),
                protocol: "TCP".to_string(),
                length: 0,
                summary: "Flags [.], length 0".to_string(),
            },
        ]
    }

    fn sample_packets_for_popup_search() -> Vec<PacketSummary> {
        vec![
            PacketSummary {
                timestamp: "1970-01-01 00:00:01.001000".to_string(),
                interface: None,
                source: "172.16.10.10.5000".to_string(),
                destination: "8.8.8.8.53".to_string(),
                protocol: "UDP".to_string(),
                length: 0,
                summary: "UDP, length 0".to_string(),
            },
            PacketSummary {
                timestamp: "1970-01-01 00:00:02.002000".to_string(),
                interface: None,
                source: "10.10.1.9.5001".to_string(),
                destination: "1.1.1.1.53".to_string(),
                protocol: "UDP".to_string(),
                length: 0,
                summary: "UDP, length 0".to_string(),
            },
            PacketSummary {
                timestamp: "1970-01-01 00:00:03.003000".to_string(),
                interface: None,
                source: "192.168.2.5.5002".to_string(),
                destination: "203.0.113.5.53".to_string(),
                protocol: "UDP".to_string(),
                length: 0,
                summary: "UDP, length 0".to_string(),
            },
        ]
    }

    fn sample_packets_for_iterative_candidates() -> Vec<PacketSummary> {
        vec![
            PacketSummary {
                timestamp: "1970-01-01 00:00:01.001000".to_string(),
                interface: None,
                source: "10.0.0.1.5000".to_string(),
                destination: "1.1.1.1.80".to_string(),
                protocol: "ICMP".to_string(),
                length: 84,
                summary: "ICMP echo request, id 1, seq 1, length 64".to_string(),
            },
            PacketSummary {
                timestamp: "1970-01-01 00:00:02.002000".to_string(),
                interface: None,
                source: "10.0.0.2.5001".to_string(),
                destination: "8.8.8.8.443".to_string(),
                protocol: "TCP".to_string(),
                length: 60,
                summary: "Flags [S], length 0".to_string(),
            },
            PacketSummary {
                timestamp: "1970-01-01 00:00:03.003000".to_string(),
                interface: None,
                source: "10.0.0.3.5002".to_string(),
                destination: "1.1.1.1.80".to_string(),
                protocol: "ICMP".to_string(),
                length: 84,
                summary: "ICMP echo reply, id 1, seq 1, length 64".to_string(),
            },
            PacketSummary {
                timestamp: "1970-01-01 00:00:04.004000".to_string(),
                interface: None,
                source: "10.0.0.4.5003".to_string(),
                destination: "1.1.1.1.443".to_string(),
                protocol: "TCP".to_string(),
                length: 60,
                summary: "Flags [.], length 0".to_string(),
            },
        ]
    }

    fn select_filter_dimension(app: &mut App, target: FilterDimension) {
        for _ in 0..app.filter_dimensions().len() {
            if app.selected_filter_dimension() == target {
                return;
            }
            app.next_filter_dimension();
        }

        for _ in 0..app.filter_dimensions().len() {
            if app.selected_filter_dimension() == target {
                return;
            }
            app.previous_filter_dimension();
        }

        panic!("failed to select filter dimension: {target:?}");
    }

    fn type_into_date_time_popup(app: &mut App, value: &str) {
        for ch in value.chars() {
            app.filter_popup_insert_char(ch);
        }
    }

    fn type_into_filter_popup_search(app: &mut App, value: &str) {
        app.start_filter_popup_search();
        for ch in value.chars() {
            app.filter_popup_search_insert_char(ch);
        }
    }

    fn move_filter_popup_to_candidate(app: &mut App, candidate: &str) {
        let candidates = app
            .filter_popup_candidates()
            .expect("filter popup should expose candidates");
        let target_index = candidates
            .iter()
            .position(|value| value == candidate)
            .unwrap_or_else(|| panic!("candidate should be present in popup: {candidate}"));
        let current_index = app.filter_popup_selected_index().unwrap_or(0);
        let steps = (target_index + candidates.len() - current_index) % candidates.len();

        for _ in 0..steps {
            app.move_down();
        }
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
    fn keybindings_popup_open_and_close_updates_visibility_state() {
        let mut app = App::new();
        assert!(!app.is_keybindings_popup_open());

        app.open_keybindings_popup();
        assert!(app.is_keybindings_popup_open());

        app.close_keybindings_popup();
        assert!(!app.is_keybindings_popup_open());
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
        assert_eq!(
            app.selected_filter_dimension(),
            *app.filter_dimensions()
                .last()
                .expect("at least one filter dimension should exist")
        );
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
    fn popup_search_filters_candidates_by_substring_match_anywhere() {
        let mut app = App::with_packets(sample_packets_for_popup_search(), String::new());

        app.open_filter_popup();
        assert!(!app.is_filter_popup_search_active());
        type_into_filter_popup_search(&mut app, "10.10");

        assert!(app.is_filter_popup_search_active());
        assert_eq!(app.filter_popup_search_query(), Some("10.10"));
        let narrowed = app
            .filter_popup_candidates()
            .expect("host popup should expose narrowed candidates");
        assert_eq!(
            narrowed,
            &["10.10.1.9".to_string(), "172.16.10.10".to_string()]
        );

        for _ in 0.."10.10".chars().count() {
            app.filter_popup_search_backspace();
        }
        assert_eq!(app.filter_popup_search_query(), Some(""));
        let widened = app
            .filter_popup_candidates()
            .expect("host popup should expose all candidates once query is cleared");
        assert_eq!(
            widened,
            &[
                "1.1.1.1".to_string(),
                "10.10.1.9".to_string(),
                "172.16.10.10".to_string(),
                "192.168.2.5".to_string(),
                "203.0.113.5".to_string(),
                "8.8.8.8".to_string(),
            ]
        );
    }

    #[test]
    fn popup_confirm_keeps_values_selected_before_search_narrowing() {
        let mut app = App::with_packets(sample_packets_for_popup_search(), String::new());

        app.open_filter_popup();
        app.toggle_filter_popup_selection();
        type_into_filter_popup_search(&mut app, "10.10");
        app.confirm_filter_popup();

        assert_eq!(app.filter_expression(), "host = 1.1.1.1");
        assert_eq!(app.packets().len(), 1);
        assert_eq!(app.packets()[0].destination, "1.1.1.1.53");
    }

    #[test]
    fn stop_filter_popup_search_disables_search_mode_without_closing_popup() {
        let mut app = App::with_packets(sample_packets_for_popup_search(), String::new());

        app.open_filter_popup();
        type_into_filter_popup_search(&mut app, "10.10");
        assert!(app.is_filter_popup_search_active());
        assert_eq!(app.filter_popup_search_query(), Some("10.10"));

        app.stop_filter_popup_search();

        assert!(app.is_filter_popup_open());
        assert!(!app.is_filter_popup_search_active());
        assert_eq!(app.filter_popup_search_query(), Some("10.10"));
    }

    #[test]
    fn popup_confirm_with_search_query_keeps_focus_in_filter_selector() {
        let mut app = App::with_packets(sample_packets_for_popup_search(), String::new());
        assert_eq!(app.focus(), FocusPane::FilterSelector);

        app.open_filter_popup();
        type_into_filter_popup_search(&mut app, "10.10");
        app.stop_filter_popup_search();
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();

        assert_eq!(app.filter_expression(), "host = 10.10.1.9");
        assert_eq!(app.focus(), FocusPane::FilterSelector);
    }

    #[test]
    fn popup_confirm_keeps_focus_in_filter_selector_after_search_mode_used_with_empty_query() {
        let mut app = App::with_packets(sample_packets_for_popup_search(), String::new());
        assert_eq!(app.focus(), FocusPane::FilterSelector);

        app.open_filter_popup();
        app.start_filter_popup_search();
        app.stop_filter_popup_search();
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();

        assert_eq!(app.filter_expression(), "host = 1.1.1.1");
        assert_eq!(app.focus(), FocusPane::FilterSelector);
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
        assert_eq!(app.focus(), FocusPane::FilterSelector);
        assert_eq!(app.packets().len(), 1);
        assert_eq!(app.packets()[0].destination, "8.8.8.8.53");
    }

    #[test]
    fn date_time_popup_confirm_moves_focus_to_packet_list() {
        let mut app = App::with_packets(sample_packets(), String::new());
        app.cycle_focus();
        assert_eq!(app.focus(), FocusPane::PacketList);

        select_filter_dimension(&mut app, FilterDimension::DateTime);
        app.open_filter_popup();
        type_into_date_time_popup(&mut app, "1970-01-01 00:00:02.002000");
        app.confirm_filter_popup();

        assert_eq!(app.focus(), FocusPane::PacketList);
        assert_eq!(
            app.filter_expression(),
            "date time >= 1970-01-01 00:00:02.002000"
        );
    }

    #[test]
    fn focus_filter_selector_moves_focus_to_filter_pane() {
        let mut app = App::with_packets(sample_packets(), String::new());
        app.cycle_focus();
        assert_eq!(app.focus(), FocusPane::PacketList);

        app.focus_filter_selector();

        assert_eq!(app.focus(), FocusPane::FilterSelector);
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

        select_filter_dimension(&mut app, FilterDimension::Protocol);
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
    fn protocol_popup_includes_arp_candidate_when_packets_contain_arp() {
        let mut app = App::with_packets(sample_packets_with_arp(), String::new());

        select_filter_dimension(&mut app, FilterDimension::Protocol);
        app.open_filter_popup();
        let candidates = app
            .filter_popup_candidates()
            .expect("protocol popup should expose candidates");

        assert_eq!(
            candidates,
            &["arp".to_string(), "tcp".to_string(), "udp".to_string()]
        );
    }

    #[test]
    fn protocol_filter_matches_arp_packets() {
        let mut app = App::with_packets(sample_packets_with_arp(), String::new());

        select_filter_dimension(&mut app, FilterDimension::Protocol);
        app.open_filter_popup();
        move_filter_popup_to_candidate(&mut app, "arp");
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();

        assert_eq!(app.filter_expression(), "protocol = arp");
        assert_eq!(app.packets().len(), 1);
        assert_eq!(app.packets()[0].protocol, "ARP");
    }

    #[test]
    fn host_popup_candidates_are_narrowed_by_active_protocol_filter() {
        let mut app = App::with_packets(sample_packets_for_iterative_candidates(), String::new());

        select_filter_dimension(&mut app, FilterDimension::Protocol);
        app.open_filter_popup();
        move_filter_popup_to_candidate(&mut app, "icmp");
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();
        assert_eq!(app.filter_expression(), "protocol = icmp");

        select_filter_dimension(&mut app, FilterDimension::Host);
        app.open_filter_popup();
        let candidates = app
            .filter_popup_candidates()
            .expect("host popup should expose candidates");

        assert_eq!(
            candidates,
            &[
                "1.1.1.1".to_string(),
                "10.0.0.1".to_string(),
                "10.0.0.3".to_string(),
            ]
        );
    }

    #[test]
    fn protocol_popup_candidates_respect_other_filters_while_editing_protocol_dimension() {
        let mut app = App::with_packets(sample_packets_for_iterative_candidates(), String::new());

        select_filter_dimension(&mut app, FilterDimension::Host);
        app.open_filter_popup();
        move_filter_popup_to_candidate(&mut app, "1.1.1.1");
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();
        assert_eq!(app.filter_expression(), "host = 1.1.1.1");

        select_filter_dimension(&mut app, FilterDimension::Protocol);
        app.open_filter_popup();
        move_filter_popup_to_candidate(&mut app, "icmp");
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();
        assert_eq!(
            app.filter_expression(),
            "host = 1.1.1.1 and protocol = icmp"
        );

        app.open_filter_popup();
        let candidates = app
            .filter_popup_candidates()
            .expect("protocol popup should expose candidates");
        assert_eq!(candidates, &["icmp".to_string(), "tcp".to_string()]);
    }

    #[test]
    fn interface_popup_lists_unique_interfaces() {
        let mut app = App::with_packets(sample_packets_with_interfaces(), String::new());

        select_filter_dimension(&mut app, FilterDimension::Interface);
        assert_eq!(app.selected_filter_dimension(), FilterDimension::Interface);

        app.open_filter_popup();
        let candidates = app
            .filter_popup_candidates()
            .expect("interface popup should expose candidates");

        assert_eq!(candidates, &["en0".to_string(), "utun0".to_string()]);
    }

    #[test]
    fn interface_filter_matches_selected_interface_only() {
        let mut app = App::with_packets(sample_packets_with_interfaces(), String::new());

        select_filter_dimension(&mut app, FilterDimension::Interface);
        app.open_filter_popup();
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();

        assert_eq!(app.filter_expression(), "interface = en0");
        assert_eq!(app.packets().len(), 2);
        assert!(app
            .packets()
            .iter()
            .all(|packet| packet.interface.as_deref() == Some("en0")));
    }

    #[test]
    fn source_port_popup_lists_only_source_ports() {
        let mut app = App::with_packets(sample_packets(), String::new());

        select_filter_dimension(&mut app, FilterDimension::SourcePort);
        assert_eq!(app.selected_filter_dimension(), FilterDimension::SourcePort);

        app.open_filter_popup();
        let candidates = app
            .filter_popup_candidates()
            .expect("source port popup should expose candidates");
        assert_eq!(
            candidates,
            &[
                "34211".to_string(),
                "51544".to_string(),
                "60000".to_string(),
            ]
        );
    }

    #[test]
    fn port_popup_lists_ports_in_numeric_order() {
        let mut app = App::with_packets(sample_packets(), String::new());

        select_filter_dimension(&mut app, FilterDimension::Port);
        assert_eq!(app.selected_filter_dimension(), FilterDimension::Port);

        app.open_filter_popup();
        let candidates = app
            .filter_popup_candidates()
            .expect("port popup should expose candidates");
        assert_eq!(
            candidates,
            &[
                "53".to_string(),
                "443".to_string(),
                "34211".to_string(),
                "51544".to_string(),
                "60000".to_string(),
            ]
        );
    }

    #[test]
    fn destination_port_popup_lists_only_destination_ports() {
        let mut app = App::with_packets(sample_packets(), String::new());

        select_filter_dimension(&mut app, FilterDimension::DestinationPort);
        assert_eq!(
            app.selected_filter_dimension(),
            FilterDimension::DestinationPort
        );

        app.open_filter_popup();
        let candidates = app
            .filter_popup_candidates()
            .expect("destination port popup should expose candidates");
        assert_eq!(candidates, &["53".to_string(), "443".to_string()]);
    }

    #[test]
    fn source_port_filter_matches_source_endpoint_only() {
        let mut app = App::with_packets(sample_packets(), String::new());

        select_filter_dimension(&mut app, FilterDimension::SourcePort);
        app.open_filter_popup();
        app.move_down();
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();

        assert_eq!(app.filter_expression(), "src port = 51544");
        assert_eq!(app.packets().len(), 1);
        assert_eq!(app.packets()[0].source, "10.0.0.12.51544");
    }

    #[test]
    fn destination_port_filter_matches_destination_endpoint_only() {
        let mut app = App::with_packets(sample_packets(), String::new());

        select_filter_dimension(&mut app, FilterDimension::DestinationPort);
        app.open_filter_popup();
        app.move_down();
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();

        assert_eq!(app.filter_expression(), "dst port = 443");
        assert_eq!(app.packets().len(), 2);
        assert!(app
            .packets()
            .iter()
            .all(|packet| packet.destination.ends_with(".443")));
    }

    #[test]
    fn ip_version_popup_lists_ipv4_and_ipv6_candidates() {
        let mut app = App::with_packets(sample_packets_with_ipv6(), String::new());

        select_filter_dimension(&mut app, FilterDimension::IpVersion);
        assert_eq!(app.selected_filter_dimension(), FilterDimension::IpVersion);

        app.open_filter_popup();
        let candidates = app
            .filter_popup_candidates()
            .expect("ip version popup should expose candidates");
        assert_eq!(candidates, &["ipv4".to_string(), "ipv6".to_string()]);
    }

    #[test]
    fn ip_version_filter_matches_only_requested_address_family() {
        let mut app = App::with_packets(sample_packets_with_ipv6(), String::new());

        select_filter_dimension(&mut app, FilterDimension::IpVersion);
        app.open_filter_popup();
        app.move_down();
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();

        assert_eq!(app.filter_expression(), "ip version = ipv6");
        assert_eq!(app.packets().len(), 1);
        assert!(app.packets()[0].source.contains(':'));
    }

    #[test]
    fn date_time_popup_supports_optional_start_and_end_inputs() {
        let mut app = App::with_packets(sample_packets(), String::new());

        select_filter_dimension(&mut app, FilterDimension::DateTime);
        assert_eq!(app.selected_filter_dimension(), FilterDimension::DateTime);

        app.open_filter_popup();
        assert!(app.is_filter_popup_date_time());
        assert_eq!(app.filter_popup_candidates(), None);
        assert_eq!(app.filter_popup_date_time_start_input(), Some(""));
        assert_eq!(app.filter_popup_date_time_end_input(), Some(""));
        assert_eq!(
            app.filter_popup_date_time_active_field(),
            Some(DateTimePopupField::Start)
        );
    }

    #[test]
    fn date_time_filter_applies_inclusive_start_and_end_bounds() {
        let mut app = App::with_packets(sample_packets(), String::new());

        select_filter_dimension(&mut app, FilterDimension::DateTime);
        app.open_filter_popup();
        type_into_date_time_popup(&mut app, "1970-01-01 00:00:02.002000");
        app.filter_popup_switch_date_time_field();
        type_into_date_time_popup(&mut app, "1970-01-01 00:00:03.003000");
        app.confirm_filter_popup();

        assert_eq!(
            app.filter_expression(),
            "date time between [1970-01-01 00:00:02.002000 .. 1970-01-01 00:00:03.003000]"
        );
        assert_eq!(app.packets().len(), 2);
    }

    #[test]
    fn date_time_filter_supports_only_start_or_only_end_bound() {
        let mut start_only_app = App::with_packets(sample_packets(), String::new());
        select_filter_dimension(&mut start_only_app, FilterDimension::DateTime);
        start_only_app.open_filter_popup();
        type_into_date_time_popup(&mut start_only_app, "1970-01-01 00:00:02.002000");
        start_only_app.confirm_filter_popup();

        assert_eq!(
            start_only_app.filter_expression(),
            "date time >= 1970-01-01 00:00:02.002000"
        );
        assert_eq!(start_only_app.packets().len(), 2);

        let mut end_only_app = App::with_packets(sample_packets(), String::new());
        select_filter_dimension(&mut end_only_app, FilterDimension::DateTime);
        end_only_app.open_filter_popup();
        end_only_app.filter_popup_switch_date_time_field();
        type_into_date_time_popup(&mut end_only_app, "1970-01-01 00:00:01.001000");
        end_only_app.confirm_filter_popup();

        assert_eq!(
            end_only_app.filter_expression(),
            "date time <= 1970-01-01 00:00:01.001000"
        );
        assert_eq!(end_only_app.packets().len(), 1);
    }

    #[test]
    fn traffic_class_popup_lists_unicast_multicast_and_broadcast() {
        let mut app = App::with_packets(sample_packets_with_traffic_classes(), String::new());

        select_filter_dimension(&mut app, FilterDimension::TrafficClass);
        assert_eq!(
            app.selected_filter_dimension(),
            FilterDimension::TrafficClass
        );

        app.open_filter_popup();
        let candidates = app
            .filter_popup_candidates()
            .expect("traffic class popup should expose candidates");
        assert_eq!(
            candidates,
            &[
                "broadcast".to_string(),
                "multicast".to_string(),
                "unicast".to_string(),
            ]
        );
    }

    #[test]
    fn traffic_class_filter_matches_broadcast_packets() {
        let mut app = App::with_packets(sample_packets_with_traffic_classes(), String::new());

        select_filter_dimension(&mut app, FilterDimension::TrafficClass);
        app.open_filter_popup();
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();

        assert_eq!(app.filter_expression(), "traffic class = broadcast");
        assert_eq!(app.packets().len(), 1);
        assert!(app.packets()[0].destination.starts_with("255.255.255.255"));
    }

    #[test]
    fn icmp_type_popup_lists_unique_types() {
        let mut app = App::with_packets(sample_packets_with_icmp_types(), String::new());

        select_filter_dimension(&mut app, FilterDimension::IcmpType);
        assert_eq!(app.selected_filter_dimension(), FilterDimension::IcmpType);

        app.open_filter_popup();
        let candidates = app
            .filter_popup_candidates()
            .expect("icmp type popup should expose candidates");
        assert_eq!(
            candidates,
            &[
                "echo reply".to_string(),
                "echo request".to_string(),
                "neighbor solicitation".to_string(),
            ]
        );
    }

    #[test]
    fn icmp_type_filter_matches_selected_type_only() {
        let mut app = App::with_packets(sample_packets_with_icmp_types(), String::new());

        select_filter_dimension(&mut app, FilterDimension::IcmpType);
        app.open_filter_popup();
        app.move_down();
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();

        assert_eq!(app.filter_expression(), "icmp type = echo request");
        assert_eq!(app.packets().len(), 1);
        assert!(app.packets()[0].summary.starts_with("ICMP echo request"));
    }

    #[test]
    fn tcp_flags_popup_lists_unique_available_flags() {
        let mut app = App::with_packets(sample_packets(), String::new());

        select_filter_dimension(&mut app, FilterDimension::TcpFlags);
        assert_eq!(app.selected_filter_dimension(), FilterDimension::TcpFlags);

        app.open_filter_popup();
        let candidates = app
            .filter_popup_candidates()
            .expect("tcp flags popup should expose candidates");
        assert_eq!(
            candidates,
            &[
                "NS".to_string(),
                "CWR".to_string(),
                "ECE".to_string(),
                "URG".to_string(),
                "ACK".to_string(),
                "PSH".to_string(),
                "RST".to_string(),
                "SYN".to_string(),
                "FIN".to_string(),
            ]
        );
    }

    #[test]
    fn tcp_flags_filter_matches_packets_by_selected_flag() {
        let mut app = App::with_packets(sample_packets(), String::new());

        select_filter_dimension(&mut app, FilterDimension::TcpFlags);
        assert_eq!(app.selected_filter_dimension(), FilterDimension::TcpFlags);

        app.open_filter_popup();
        let syn_index = app
            .filter_popup_candidates()
            .expect("tcp flags popup should expose candidates")
            .iter()
            .position(|candidate| candidate == "SYN")
            .expect("SYN flag should be present");
        for _ in 0..syn_index {
            app.move_down();
        }
        assert_eq!(app.filter_popup_selected_index(), Some(syn_index));
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();

        assert_eq!(app.filter_expression(), "tcp flags = SYN");
        assert_eq!(app.packets().len(), 1);
        assert!(app.packets()[0].summary.contains("Flags [S]"));
    }

    #[test]
    fn tcp_flags_filter_matches_only_exact_ack_without_extra_flags() {
        let mut app = App::with_packets(sample_packets_with_tcp_flag_combinations(), String::new());

        select_filter_dimension(&mut app, FilterDimension::TcpFlags);
        app.open_filter_popup();
        move_filter_popup_to_candidate(&mut app, "ACK");
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();

        assert_eq!(app.filter_expression(), "tcp flags = ACK");
        assert_eq!(app.packets().len(), 1);
        assert!(app.packets()[0].summary.contains("Flags [.]"));
    }

    #[test]
    fn tcp_flags_filter_matches_only_exact_syn_ack_set() {
        let mut app = App::with_packets(sample_packets_with_tcp_flag_combinations(), String::new());

        select_filter_dimension(&mut app, FilterDimension::TcpFlags);
        app.open_filter_popup();
        move_filter_popup_to_candidate(&mut app, "ACK");
        app.toggle_filter_popup_selection();
        move_filter_popup_to_candidate(&mut app, "SYN");
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();

        assert_eq!(app.filter_expression(), "tcp flags in [ACK, SYN]");
        assert_eq!(app.packets().len(), 1);
        assert!(app.packets()[0].summary.contains("Flags [S.]"));
    }

    #[test]
    fn popup_confirm_without_selections_clears_active_filter() {
        let mut app = App::with_packets(sample_packets(), String::new());

        select_filter_dimension(&mut app, FilterDimension::Protocol);

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
    fn popup_clear_selection_removes_all_checked_values_for_dimension() {
        let mut app = App::with_packets(sample_packets(), String::new());

        select_filter_dimension(&mut app, FilterDimension::Protocol);
        assert_eq!(app.selected_filter_dimension(), FilterDimension::Protocol);

        app.open_filter_popup();
        app.toggle_filter_popup_selection();
        app.move_down();
        app.toggle_filter_popup_selection();
        app.clear_filter_popup_selection();
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

        select_filter_dimension(&mut app, FilterDimension::Protocol);
        assert_eq!(app.selected_filter_dimension(), FilterDimension::Protocol);

        app.open_filter_popup();
        app.move_down();
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();

        assert_eq!(app.filter_expression(), "host = 1.1.1.1 and protocol = tcp");
        assert_eq!(app.packets().len(), 2);
    }

    #[test]
    fn clear_all_filters_resets_all_categories() {
        let mut app = App::with_packets(sample_packets(), String::new());

        app.open_filter_popup();
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();
        assert_eq!(app.filter_expression(), "host = 1.1.1.1");

        select_filter_dimension(&mut app, FilterDimension::Protocol);
        app.open_filter_popup();
        app.move_down();
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();
        assert_eq!(app.filter_expression(), "host = 1.1.1.1 and protocol = tcp");
        assert_eq!(app.packets().len(), 2);

        app.clear_all_filters();

        assert_eq!(app.filter_expression(), "");
        assert_eq!(app.packets().len(), 3);
    }

    #[test]
    fn clear_selected_filter_dimension_only_clears_current_category() {
        let mut app = App::with_packets(sample_packets(), String::new());

        app.open_filter_popup();
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();
        assert_eq!(app.filter_expression(), "host = 1.1.1.1");

        select_filter_dimension(&mut app, FilterDimension::Protocol);
        assert_eq!(app.selected_filter_dimension(), FilterDimension::Protocol);

        app.open_filter_popup();
        app.move_down();
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();
        assert_eq!(app.filter_expression(), "host = 1.1.1.1 and protocol = tcp");
        assert_eq!(app.packets().len(), 2);

        app.clear_selected_filter_dimension();

        assert_eq!(app.filter_expression(), "host = 1.1.1.1");
        assert_eq!(app.packets().len(), 2);
        assert!(app.is_filter_dimension_active(FilterDimension::Host));
        assert!(!app.is_filter_dimension_active(FilterDimension::Protocol));
    }

    #[test]
    fn filter_dimension_active_state_reflects_applied_selections() {
        let mut app = App::with_packets(sample_packets(), String::new());

        assert!(!app.is_filter_dimension_active(FilterDimension::Host));
        assert!(!app.is_filter_dimension_active(FilterDimension::Protocol));

        app.open_filter_popup();
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();
        assert!(app.is_filter_dimension_active(FilterDimension::Host));
        assert!(!app.is_filter_dimension_active(FilterDimension::Protocol));

        select_filter_dimension(&mut app, FilterDimension::Protocol);
        assert_eq!(app.selected_filter_dimension(), FilterDimension::Protocol);

        app.open_filter_popup();
        app.move_down();
        app.toggle_filter_popup_selection();
        app.confirm_filter_popup();

        assert!(app.is_filter_dimension_active(FilterDimension::Host));
        assert!(app.is_filter_dimension_active(FilterDimension::Protocol));

        app.clear_all_filters();
        assert!(!app.is_filter_dimension_active(FilterDimension::Host));
        assert!(!app.is_filter_dimension_active(FilterDimension::Protocol));
    }

    #[test]
    fn endpoint_host_keeps_plain_ipv4_without_port() {
        assert_eq!(endpoint_host("1.1.1.1"), "1.1.1.1");
        assert_eq!(endpoint_host("192.168.0.242"), "192.168.0.242");
    }

    #[test]
    fn endpoint_port_ignores_plain_ipv4_without_port() {
        assert_eq!(endpoint_port("1.1.1.1"), None);
        assert_eq!(endpoint_port("192.168.0.242"), None);
    }

    #[test]
    fn host_popup_includes_full_plain_ipv4_hosts() {
        let packets = vec![PacketSummary {
            timestamp: "1970-01-01 00:00:04.004000".to_string(),
            interface: None,
            source: "192.168.0.242".to_string(),
            destination: "1.1.1.1".to_string(),
            protocol: "ICMP".to_string(),
            length: 84,
            summary: "ICMP echo request".to_string(),
        }];
        let mut app = App::with_packets(packets, String::new());

        app.open_filter_popup();
        let candidates = app
            .filter_popup_candidates()
            .expect("host popup should expose candidates");

        assert_eq!(
            candidates,
            &["1.1.1.1".to_string(), "192.168.0.242".to_string()]
        );
    }
}

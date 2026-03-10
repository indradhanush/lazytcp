use crate::capture::CaptureState;
use crate::domain::{FilterDimension, PacketSummary};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPane {
    FilterSelector,
    PacketList,
    FilterInput,
    PacketDetail,
}

pub struct App {
    should_quit: bool,
    focus: FocusPane,
    packets: Vec<PacketSummary>,
    selected_packet: usize,
    filter_dimensions: Vec<FilterDimension>,
    selected_filter_dimension: usize,
    filter_input: String,
    capture_state: CaptureState,
}

impl App {
    pub fn new() -> Self {
        Self::with_packets(Vec::new(), String::new())
    }

    pub fn with_packets(packets: Vec<PacketSummary>, filter_input: String) -> Self {
        Self {
            should_quit: false,
            focus: FocusPane::PacketList,
            packets,
            selected_packet: 0,
            filter_dimensions: FilterDimension::ALL.to_vec(),
            selected_filter_dimension: 0,
            filter_input,
            capture_state: CaptureState::Idle,
        }
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

    pub fn filter_input(&self) -> &str {
        &self.filter_input
    }

    pub fn insert_filter_input_char(&mut self, ch: char) {
        self.filter_input.push(ch);
    }

    pub fn backspace_filter_input(&mut self) {
        self.filter_input.pop();
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

        self.selected_filter_dimension =
            (self.selected_filter_dimension + 1).min(self.filter_dimensions.len() - 1);
    }

    pub fn previous_filter_dimension(&mut self) {
        self.selected_filter_dimension = self.selected_filter_dimension.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        match self.focus {
            FocusPane::FilterSelector => self.next_filter_dimension(),
            FocusPane::PacketList => self.next_packet(),
            FocusPane::FilterInput | FocusPane::PacketDetail => {}
        }
    }

    pub fn move_up(&mut self) {
        match self.focus {
            FocusPane::FilterSelector => self.previous_filter_dimension(),
            FocusPane::PacketList => self.previous_packet(),
            FocusPane::FilterInput | FocusPane::PacketDetail => {}
        }
    }

    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            FocusPane::PacketList => FocusPane::FilterSelector,
            FocusPane::FilterSelector => FocusPane::PacketDetail,
            FocusPane::PacketDetail => FocusPane::FilterInput,
            FocusPane::FilterInput => FocusPane::PacketList,
        };
    }

    pub fn reverse_cycle_focus(&mut self) {
        self.focus = match self.focus {
            FocusPane::PacketList => FocusPane::FilterInput,
            FocusPane::FilterInput => FocusPane::PacketDetail,
            FocusPane::PacketDetail => FocusPane::FilterSelector,
            FocusPane::FilterSelector => FocusPane::PacketList,
        };
    }
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
        ]
    }

    #[test]
    fn cycle_focus_wraps_back_to_packet_list() {
        let mut app = App::new();
        assert_eq!(app.focus(), FocusPane::PacketList);

        app.cycle_focus();
        assert_eq!(app.focus(), FocusPane::FilterSelector);

        app.cycle_focus();
        assert_eq!(app.focus(), FocusPane::PacketDetail);

        app.cycle_focus();
        assert_eq!(app.focus(), FocusPane::FilterInput);

        app.cycle_focus();
        assert_eq!(app.focus(), FocusPane::PacketList);
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
    fn insert_filter_input_char_appends_text() {
        let mut app = App::new();
        app.insert_filter_input_char('u');
        app.insert_filter_input_char('d');
        app.insert_filter_input_char('p');

        assert_eq!(app.filter_input(), "udp");
    }

    #[test]
    fn backspace_filter_input_removes_last_character() {
        let mut app = App::with_packets(Vec::new(), "tcp".to_string());
        app.backspace_filter_input();

        assert_eq!(app.filter_input(), "tc");
    }

    #[test]
    fn move_down_in_packet_list_focus_advances_packet_selection() {
        let mut app = App::with_packets(sample_packets(), String::new());

        app.move_down();

        assert_eq!(app.selected_packet_index(), 1);
    }

    #[test]
    fn move_down_in_filter_selector_focus_advances_filter_selection() {
        let mut app = App::new();
        app.cycle_focus();
        assert_eq!(app.focus(), FocusPane::FilterSelector);

        app.move_down();

        assert_eq!(app.selected_filter_dimension(), FilterDimension::Source);
    }

    #[test]
    fn reverse_cycle_focus_wraps_back_to_packet_list() {
        let mut app = App::new();
        assert_eq!(app.focus(), FocusPane::PacketList);

        app.reverse_cycle_focus();
        assert_eq!(app.focus(), FocusPane::FilterInput);

        app.reverse_cycle_focus();
        assert_eq!(app.focus(), FocusPane::PacketDetail);

        app.reverse_cycle_focus();
        assert_eq!(app.focus(), FocusPane::FilterSelector);

        app.reverse_cycle_focus();
        assert_eq!(app.focus(), FocusPane::PacketList);
    }
}

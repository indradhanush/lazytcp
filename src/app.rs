use crate::capture::CaptureState;
use crate::domain::PacketSummary;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPane {
    PacketList,
    FilterInput,
    PacketDetail,
}

pub struct App {
    should_quit: bool,
    focus: FocusPane,
    packets: Vec<PacketSummary>,
    selected_packet: usize,
    filter_input: String,
    capture_state: CaptureState,
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            focus: FocusPane::PacketList,
            packets: PacketSummary::demo_rows(),
            selected_packet: 0,
            filter_input: "host 10.0.0.12 and tcp".to_string(),
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

    pub fn filter_input(&self) -> &str {
        &self.filter_input
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

    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            FocusPane::PacketList => FocusPane::FilterInput,
            FocusPane::FilterInput => FocusPane::PacketDetail,
            FocusPane::PacketDetail => FocusPane::PacketList,
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
    use super::{App, FocusPane};

    #[test]
    fn cycle_focus_wraps_back_to_packet_list() {
        let mut app = App::new();
        assert_eq!(app.focus(), FocusPane::PacketList);

        app.cycle_focus();
        assert_eq!(app.focus(), FocusPane::FilterInput);

        app.cycle_focus();
        assert_eq!(app.focus(), FocusPane::PacketDetail);

        app.cycle_focus();
        assert_eq!(app.focus(), FocusPane::PacketList);
    }

    #[test]
    fn next_packet_stops_at_last_row() {
        let mut app = App::new();
        let last_index = app.packets().len() - 1;

        for _ in 0..(app.packets().len() + 3) {
            app.next_packet();
        }

        assert_eq!(app.selected_packet_index(), last_index);
    }

    #[test]
    fn previous_packet_stops_at_zero() {
        let mut app = App::new();
        app.previous_packet();

        assert_eq!(app.selected_packet_index(), 0);
    }
}

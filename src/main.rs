use std::io;
use std::path::PathBuf;
use std::process;
use std::time::{Duration, Instant};

use crossterm::cursor::DisableBlinking;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::style::Print;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use lazytcp::api::{TcpdumpApi, TcpdumpReadRequest};
use lazytcp::app::{App, FocusPane};
use lazytcp::domain::PacketSummary;
use lazytcp::ui;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

type AppResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Clone)]
struct CliArgs {
    pcap_path: PathBuf,
}

#[derive(Debug, Clone)]
enum ParsedArgs {
    Run(CliArgs),
    Help,
}

#[derive(Debug, Clone)]
enum CliError {
    Usage(String),
    Runtime(String),
}

impl CliError {
    fn usage(message: impl Into<String>) -> Self {
        Self::Usage(message.into())
    }

    fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime(message.into())
    }

    fn exit_code(&self) -> i32 {
        match self {
            Self::Usage(_) => 2,
            Self::Runtime(_) => 1,
        }
    }

    fn message(&self) -> &str {
        match self {
            Self::Usage(message) | Self::Runtime(message) => message,
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{}", error.message());
        process::exit(error.exit_code());
    }
}

fn run() -> Result<(), CliError> {
    let args = match parse_args()? {
        ParsedArgs::Help => {
            println!("{}", usage());
            return Ok(());
        }
        ParsedArgs::Run(args) => args,
    };

    let packets = load_packets(&args)?;
    let filter_input = String::new();

    let mut terminal = init_terminal()
        .map_err(|err| CliError::runtime(format!("error: failed to initialize terminal: {err}")))?;

    let run_result = run_app(&mut terminal, packets, filter_input)
        .map_err(|err| CliError::runtime(format!("error: {err}")));

    let restore_result = restore_terminal(&mut terminal);
    if let Err(err) = restore_result {
        eprintln!("error: failed to restore terminal state: {err}");
    }

    run_result
}

fn parse_args() -> Result<ParsedArgs, CliError> {
    parse_args_from(std::env::args().skip(1))
}

fn parse_args_from<I>(args: I) -> Result<ParsedArgs, CliError>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();
    let Some(first) = args.next() else {
        return Err(CliError::usage(format!(
            "error: missing required argument <pcap-file>\n\n{}",
            usage()
        )));
    };

    if first == "-h" || first == "--help" {
        return Ok(ParsedArgs::Help);
    }

    if first.starts_with('-') {
        return Err(CliError::usage(format!(
            "error: unknown option '{}'\n\n{}",
            first,
            usage()
        )));
    }

    if let Some(unexpected) = args.next() {
        return Err(CliError::usage(format!(
            "error: unexpected argument '{}'\n\n{}",
            unexpected,
            usage()
        )));
    }

    Ok(ParsedArgs::Run(CliArgs {
        pcap_path: PathBuf::from(first),
    }))
}

fn usage() -> String {
    "usage: lazytcp <pcap-file>\nexample: lazytcp capture.pcap".to_string()
}

fn load_packets(args: &CliArgs) -> Result<Vec<PacketSummary>, CliError> {
    if !args.pcap_path.exists() {
        return Err(CliError::runtime(format!(
            "error: pcap file not found: {}",
            args.pcap_path.display()
        )));
    }

    let request = TcpdumpReadRequest {
        pcap_path: &args.pcap_path,
        filter_args: &[],
    };

    TcpdumpApi::default()
        .read_pcap(request)
        .map_err(|err| CliError::runtime(format!("error: {err}")))
}

fn init_terminal() -> AppResult<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    terminal.hide_cursor()?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    let _ = reset_cursor_color(terminal);
    let _ = terminal.backend_mut().execute(DisableBlinking);
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    packets: Vec<PacketSummary>,
    filter_input: String,
) -> AppResult<()> {
    let mut app = App::with_packets(packets, filter_input);
    let mut date_time_cursor_visible = true;
    let mut last_blink_toggle_at = Instant::now();
    let mut date_time_cursor_color_applied = false;

    while !app.should_quit() {
        terminal.draw(|frame| ui::render(frame, &app))?;
        sync_date_time_cursor_mode(
            terminal,
            &app,
            &mut date_time_cursor_visible,
            &mut last_blink_toggle_at,
            &mut date_time_cursor_color_applied,
        )?;

        if event::poll(Duration::from_millis(100))? {
            handle_event(&mut app)?;
        }
    }

    Ok(())
}

fn sync_date_time_cursor_mode(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &App,
    date_time_cursor_visible: &mut bool,
    last_blink_toggle_at: &mut Instant,
    date_time_cursor_color_applied: &mut bool,
) -> io::Result<()> {
    let should_activate = app.is_filter_popup_open() && app.is_filter_popup_date_time();
    const BLINK_INTERVAL: Duration = Duration::from_millis(500);

    if !should_activate {
        if *date_time_cursor_color_applied {
            reset_cursor_color(terminal)?;
            *date_time_cursor_color_applied = false;
        }
        terminal.hide_cursor()?;
        *date_time_cursor_visible = true;
        *last_blink_toggle_at = Instant::now();
        return Ok(());
    }

    if !*date_time_cursor_color_applied {
        set_cursor_color_grey(terminal)?;
        *date_time_cursor_color_applied = true;
    }

    if last_blink_toggle_at.elapsed() >= BLINK_INTERVAL {
        *date_time_cursor_visible = !*date_time_cursor_visible;
        *last_blink_toggle_at = Instant::now();
    }

    if *date_time_cursor_visible {
        terminal.show_cursor()?;
    } else {
        terminal.hide_cursor()?;
    }

    Ok(())
}

fn set_cursor_color_grey(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    terminal
        .backend_mut()
        .execute(Print("\x1b]12;#808080\x07"))?;
    Ok(())
}

fn reset_cursor_color(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    terminal.backend_mut().execute(Print("\x1b]112\x07"))?;
    Ok(())
}

fn handle_event(app: &mut App) -> AppResult<()> {
    if let Event::Key(key) = event::read()? {
        if key.kind == KeyEventKind::Press {
            handle_key_press(app, key.code, key.modifiers);
        }
    }

    Ok(())
}

fn handle_key_press(app: &mut App, key_code: KeyCode, modifiers: KeyModifiers) {
    if app.is_keybindings_popup_open() {
        match key_code {
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => app.quit(),
            KeyCode::Char('q') => app.quit(),
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('?') => app.close_keybindings_popup(),
            _ => {}
        }
        return;
    }

    if key_code == KeyCode::Char('?') {
        app.open_keybindings_popup();
        return;
    }

    if app.is_filter_popup_open() {
        if app.is_filter_popup_date_time() {
            match key_code {
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => app.quit(),
                KeyCode::Char('q') => app.quit(),
                KeyCode::Char('j') | KeyCode::Down => app.move_down(),
                KeyCode::Char('k') | KeyCode::Up => app.move_up(),
                KeyCode::Tab | KeyCode::BackTab => app.filter_popup_switch_date_time_field(),
                KeyCode::Backspace => app.filter_popup_backspace(),
                KeyCode::Char('c') => app.clear_filter_popup_selection(),
                KeyCode::Char('C') => app.clear_all_filters(),
                KeyCode::Enter => app.confirm_filter_popup(),
                KeyCode::Char(ch) => app.filter_popup_insert_char(ch),
                _ if is_popup_cancel_key(key_code, modifiers) => app.close_filter_popup(),
                _ => {}
            }
        } else {
            match key_code {
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => app.quit(),
                KeyCode::Char('q') => app.quit(),
                KeyCode::Char('j') | KeyCode::Down => app.move_down(),
                KeyCode::Char('k') | KeyCode::Up => app.move_up(),
                KeyCode::Char('/') => app.start_filter_popup_search(),
                KeyCode::Backspace if app.is_filter_popup_search_active() => {
                    app.filter_popup_search_backspace()
                }
                KeyCode::Char(ch) if app.is_filter_popup_search_active() => {
                    app.filter_popup_search_insert_char(ch)
                }
                KeyCode::Char(' ') => app.toggle_filter_popup_selection(),
                KeyCode::Char('c') => app.clear_filter_popup_selection(),
                KeyCode::Char('C') if !app.is_filter_popup_search_active() => {
                    app.clear_all_filters()
                }
                KeyCode::Enter if app.is_filter_popup_search_active() => {
                    app.stop_filter_popup_search()
                }
                KeyCode::Enter => app.confirm_filter_popup(),
                _ if is_popup_cancel_key(key_code, modifiers) => app.close_filter_popup(),
                _ => {}
            }
        }
        return;
    }

    match key_code {
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => app.quit(),
        KeyCode::Enter if app.focus() == FocusPane::FilterInput => app.focus_packet_list(),
        KeyCode::Enter if app.focus() == FocusPane::FilterSelector => app.open_filter_popup(),
        KeyCode::Char('c') if app.focus() == FocusPane::FilterSelector => {
            app.clear_selected_filter_dimension()
        }
        KeyCode::Char('q') => app.quit(),
        KeyCode::Char('C') => app.clear_all_filters(),
        KeyCode::Char('0') => app.focus_filter_selector(),
        KeyCode::Char('1') => app.focus_packet_list(),
        KeyCode::Char('/') => app.focus_filter_selector(),
        KeyCode::Char('j') | KeyCode::Down => app.move_down(),
        KeyCode::Char('k') | KeyCode::Up => app.move_up(),
        KeyCode::Tab => app.cycle_focus(),
        KeyCode::BackTab => app.reverse_cycle_focus(),
        _ => {}
    }
}

fn is_popup_cancel_key(code: KeyCode, modifiers: KeyModifiers) -> bool {
    matches!(code, KeyCode::Esc | KeyCode::Char('\u{1b}'))
        || (matches!(code, KeyCode::Char('[')) && modifiers.contains(KeyModifiers::CONTROL))
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyModifiers};
    use lazytcp::app::{App, FocusPane};
    use lazytcp::domain::{FilterDimension, PacketSummary};

    use super::{handle_key_press, is_popup_cancel_key, parse_args_from, CliError, ParsedArgs};

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

    fn press_key(app: &mut App, key_code: KeyCode) {
        handle_key_press(app, key_code, KeyModifiers::NONE);
    }

    fn type_text(app: &mut App, value: &str) {
        for ch in value.chars() {
            press_key(app, KeyCode::Char(ch));
        }
    }

    fn select_filter_dimension(app: &mut App, target: FilterDimension) {
        for _ in 0..app.filter_dimensions().len() {
            if app.selected_filter_dimension() == target {
                return;
            }
            app.next_filter_dimension();
        }
        panic!("failed to select filter dimension: {target:?}");
    }

    #[test]
    fn missing_pcap_argument_returns_usage_error() {
        let result = parse_args_from(Vec::<String>::new());
        match result {
            Err(CliError::Usage(message)) => {
                assert!(message.contains("missing required argument <pcap-file>"));
                assert!(message.contains("usage: lazytcp <pcap-file>"));
            }
            other => panic!("expected usage error, got {other:?}"),
        }
    }

    #[test]
    fn extra_positional_argument_returns_usage_error() {
        let result = parse_args_from(vec!["capture.pcap".to_string(), "udp".to_string()]);
        match result {
            Err(CliError::Usage(message)) => {
                assert!(message.contains("unexpected argument 'udp'"));
                assert!(message.contains("usage: lazytcp <pcap-file>"));
            }
            other => panic!("expected usage error, got {other:?}"),
        }
    }

    #[test]
    fn help_flag_returns_help_variant() {
        let result = parse_args_from(vec!["--help".to_string()]).expect("help should parse");
        assert!(matches!(result, ParsedArgs::Help));
    }

    #[test]
    fn popup_cancel_key_matches_escape_variants() {
        assert!(is_popup_cancel_key(KeyCode::Esc, KeyModifiers::NONE));
        assert!(is_popup_cancel_key(
            KeyCode::Char('\u{1b}'),
            KeyModifiers::NONE
        ));
        assert!(is_popup_cancel_key(
            KeyCode::Char('['),
            KeyModifiers::CONTROL
        ));
        assert!(!is_popup_cancel_key(KeyCode::Char('['), KeyModifiers::NONE));
    }

    #[test]
    fn given_no_search_when_confirm_multiselect_popup_then_focus_stays_filter() {
        let mut app = App::with_packets(sample_packets(), String::new());

        press_key(&mut app, KeyCode::Enter);
        press_key(&mut app, KeyCode::Char(' '));
        press_key(&mut app, KeyCode::Enter);

        assert_eq!(app.focus(), FocusPane::FilterSelector);
    }

    #[test]
    fn given_search_active_when_press_enter_then_search_exits_and_popup_stays_open() {
        let mut app = App::with_packets(sample_packets(), String::new());

        press_key(&mut app, KeyCode::Enter);
        press_key(&mut app, KeyCode::Char('/'));
        type_text(&mut app, "10.0");
        press_key(&mut app, KeyCode::Enter);

        assert!(app.is_filter_popup_open());
        assert!(!app.is_filter_popup_search_active());
    }

    #[test]
    fn given_search_cleared_when_confirm_multiselect_popup_then_focus_stays_filter() {
        let mut app = App::with_packets(sample_packets(), String::new());

        press_key(&mut app, KeyCode::Enter);
        press_key(&mut app, KeyCode::Char('/'));
        type_text(&mut app, "10");
        press_key(&mut app, KeyCode::Backspace);
        press_key(&mut app, KeyCode::Backspace);
        press_key(&mut app, KeyCode::Enter);
        press_key(&mut app, KeyCode::Char(' '));
        press_key(&mut app, KeyCode::Enter);

        assert_eq!(app.focus(), FocusPane::FilterSelector);
    }

    #[test]
    fn given_date_time_popup_when_confirm_then_focus_moves_packets() {
        let mut app = App::with_packets(sample_packets(), String::new());
        select_filter_dimension(&mut app, FilterDimension::DateTime);

        press_key(&mut app, KeyCode::Enter);
        type_text(&mut app, "1970-01-01 00:00:02.002000");
        press_key(&mut app, KeyCode::Enter);

        assert_eq!(app.focus(), FocusPane::PacketList);
    }
}

use std::io;
use std::path::PathBuf;
use std::process;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
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
    filter_args: Vec<String>,
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

    Ok(ParsedArgs::Run(CliArgs {
        pcap_path: PathBuf::from(first),
        filter_args: args.collect(),
    }))
}

fn usage() -> String {
    "usage: lazytcp <pcap-file> [tcpdump-filter...]\nexample: lazytcp capture.pcap udp".to_string()
}

fn load_packets(args: &CliArgs) -> Result<Vec<PacketSummary>, CliError> {
    if !args.pcap_path.exists() {
        return Err(CliError::runtime(format!(
            "error: pcap file not found: {}",
            args.pcap_path.display()
        )));
    }

    let filter_args: Vec<&str> = args.filter_args.iter().map(String::as_str).collect();
    let request = TcpdumpReadRequest {
        pcap_path: &args.pcap_path,
        filter_args: &filter_args,
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
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
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

    while !app.should_quit() {
        terminal.draw(|frame| ui::render(frame, &app))?;

        if event::poll(Duration::from_millis(100))? {
            handle_event(&mut app)?;
        }
    }

    Ok(())
}

fn handle_event(app: &mut App) -> AppResult<()> {
    if let Event::Key(key) = event::read()? {
        if key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Char(ch) if app.focus() == FocusPane::FilterInput => {
                    app.insert_filter_input_char(ch)
                }
                KeyCode::Backspace if app.focus() == FocusPane::FilterInput => {
                    app.backspace_filter_input()
                }
                KeyCode::Enter if app.focus() == FocusPane::FilterInput => app.focus_packet_list(),
                KeyCode::Enter if app.focus() == FocusPane::FilterSelector => {
                    app.focus_filter_input()
                }
                KeyCode::Backspace if app.focus() == FocusPane::FilterSelector => {
                    app.focus_filter_input();
                    app.backspace_filter_input();
                }
                KeyCode::Char(ch)
                    if app.focus() == FocusPane::FilterSelector && ch != 'j' && ch != 'k' =>
                {
                    app.begin_filter_input_with_char(ch);
                }
                KeyCode::Char('q') => app.quit(),
                KeyCode::Char('j') | KeyCode::Down => app.move_down(),
                KeyCode::Char('k') | KeyCode::Up => app.move_up(),
                KeyCode::Tab => app.cycle_focus(),
                KeyCode::BackTab => app.reverse_cycle_focus(),
                _ => {}
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_args_from, CliError, ParsedArgs};

    #[test]
    fn missing_pcap_argument_returns_usage_error() {
        let result = parse_args_from(Vec::<String>::new());
        match result {
            Err(CliError::Usage(message)) => {
                assert!(message.contains("missing required argument <pcap-file>"));
                assert!(message.contains("usage: lazytcp <pcap-file> [tcpdump-filter...]"));
            }
            other => panic!("expected usage error, got {other:?}"),
        }
    }

    #[test]
    fn help_flag_returns_help_variant() {
        let result = parse_args_from(vec!["--help".to_string()]).expect("help should parse");
        assert!(matches!(result, ParsedArgs::Help));
    }
}

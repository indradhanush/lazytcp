use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tcpdump_tui::app::App;
use tcpdump_tui::ui;

type AppResult<T> = Result<T, Box<dyn std::error::Error>>;

fn main() -> AppResult<()> {
    let mut terminal = init_terminal()?;

    let run_result = run_app(&mut terminal);

    let restore_result = restore_terminal(&mut terminal);

    if let Err(err) = restore_result {
        eprintln!("failed to restore terminal state: {err}");
    }

    run_result
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

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> AppResult<()> {
    let mut app = App::new();

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
                KeyCode::Char('q') => app.quit(),
                KeyCode::Char('j') | KeyCode::Down => app.next_packet(),
                KeyCode::Char('k') | KeyCode::Up => app.previous_packet(),
                KeyCode::Tab => app.cycle_focus(),
                _ => {}
            }
        }
    }

    Ok(())
}

# RATATUI Local Reference for `lazytcp`

This is a secondary local reference for ratatui APIs as used in this project.
Primary reference: `~/github.com/ratatui/examples`.

Ratatui version in use: `0.29` (see `Cargo.toml`).

---

## Terminal Lifecycle

Terminal setup and teardown live in `src/main.rs`. The pattern is:

```rust
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

// Init
enable_raw_mode()?;
stdout.execute(EnterAlternateScreen)?;
let backend = CrosstermBackend::new(stdout);
let mut terminal = Terminal::new(backend)?;
terminal.clear()?;
terminal.hide_cursor()?;

// Draw loop
terminal.draw(|frame| ui::render(frame, &app))?;

// Restore
disable_raw_mode()?;
terminal.backend_mut().execute(LeaveAlternateScreen)?;
terminal.show_cursor()?;
```

**Always restore terminal state on all exit paths**, including panics or errors.
`restore_terminal` must be called even if `run_app` returns an error.

---

## Layout

`Layout` splits a `Rect` into sub-areas using constraints.

```rust
use ratatui::layout::{Constraint, Direction, Layout, Rect};

let areas = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(3),   // exact line count
        Constraint::Min(8),      // at least N lines, takes remainder
        Constraint::Percentage(50), // percent of available
    ])
    .split(frame.area());

// areas[0], areas[1], areas[2] are Rect values
```

- `Constraint::Length(n)` — fixed size.
- `Constraint::Min(n)` — minimum size; expands to fill remaining space.
- `Constraint::Percentage(n)` — proportional share (0–100).
- `Direction::Vertical` / `Direction::Horizontal` — split axis.
- `frame.area()` returns the full terminal `Rect`.

Layouts nest freely. A `Rect` from one split can be passed into another `split()`.

---

## Rendering Widgets

All rendering happens inside the closure passed to `terminal.draw`:

```rust
terminal.draw(|frame| ui::render(frame, &app))?;
```

The `frame: &mut Frame` is the only entry point for rendering.

### Stateless widgets

```rust
frame.render_widget(widget, area);
```

### Stateful widgets (List with selection)

```rust
let mut state = ListState::default();
state.select(Some(index));
frame.render_stateful_widget(list, area, &mut state);
```

`ListState` is the canonical stateful widget in this project. State is constructed
fresh each draw cycle from `App` — it is not persisted across frames.

### Cursor position (date-time popup)

```rust
frame.set_cursor_position((x, y));
```

The cursor is manually positioned after rendering the date-time popup fields.
Show/hide cursor separately via `terminal.show_cursor()` / `terminal.hide_cursor()`.

---

## Widgets Used in This Project

### `Block`

Decorates any widget with a border, title, and optional padding.

```rust
use ratatui::widgets::{Block, BorderType, Borders, Padding};
use ratatui::style::{Color, Modifier, Style};

Block::default()
    .borders(Borders::ALL)
    .title("My Pane")
    .border_type(BorderType::Thick)   // or BorderType::Plain
    .border_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    .padding(Padding::new(left, right, top, bottom))
```

`block.inner(area)` returns the `Rect` inside the borders (useful for manual
layout inside a block without re-rendering the block).

The focused-block helper in `src/ui.rs` encapsulates the yellow/thick style
for the active pane.

### `Paragraph`

Renders one or more lines of styled text.

```rust
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::text::Line;

let p = Paragraph::new(vec![
    Line::raw("plain text"),
    Line::from(vec![Span::styled("styled", style)]),
])
.block(block)
.wrap(Wrap { trim: true });

frame.render_widget(p, area);
```

- `Wrap { trim: true }` — word-wraps and strips trailing whitespace.
- `Wrap { trim: false }` — word-wraps, preserves spacing (used for date-time popup).
- `.alignment(Alignment::Right)` — right-aligns text inside the widget.

### `List` and `ListItem`

Renders a scrollable list with optional selection highlight.

```rust
use ratatui::widgets::{List, ListItem, ListState};
use ratatui::text::Line;

let items: Vec<ListItem> = values
    .iter()
    .map(|v| ListItem::new(Line::raw(v.clone())))
    .collect();

let list = List::new(items)
    .block(focused_block("Title", is_focused))
    .highlight_style(
        Style::default()
            .fg(Color::Black)
            .bg(Color::LightGreen)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(">> ");

let mut state = ListState::default();
state.select(Some(selected_index));
frame.render_stateful_widget(list, area, &mut state);
```

- `ListState::default()` with no `select()` call renders with no highlight.
- Guard against empty slices: only call `state.select(Some(index))` when the list is non-empty.

### `Clear`

Erases the background before rendering a popup overlay.

```rust
use ratatui::widgets::Clear;

frame.render_widget(Clear, popup_area);
// then render popup content on top
```

Always render `Clear` first in popup rendering functions.

### `Line` and `Span`

Building blocks for styled inline text.

```rust
use ratatui::text::{Line, Span};

// Unstyled
Line::raw("plain string")

// Mixed styled spans on one line
Line::from(vec![
    Span::styled("label: ", Style::default().fg(Color::Yellow)),
    Span::raw(value),
])
```

---

## Styles

```rust
use ratatui::style::{Color, Modifier, Style};

let style = Style::default()
    .fg(Color::Black)
    .bg(Color::LightGreen)
    .add_modifier(Modifier::BOLD);

// Common modifiers: BOLD, DIM
// Common colors: Black, DarkGray, Yellow, LightGreen, LightBlue, LightYellow, White, Green
```

Styles compose: set only what differs from the default.

---

## Centered Popup Helper

The `centered_rect` helper in `src/ui.rs` places a percentage-sized `Rect`
in the center of a parent area:

```rust
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    // splits vertically to center percent_y, then horizontally to center percent_x
}
```

Usage:
```rust
let popup_area = centered_rect(60, 70, frame.area());
frame.render_widget(Clear, popup_area);
// render popup content at popup_area
```

---

## Event Polling

Events are read with `crossterm`, not ratatui directly:

```rust
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use std::time::Duration;

if event::poll(Duration::from_millis(100))? {
    if let Event::Key(key) = event::read()? {
        if key.kind == KeyEventKind::Press {
            // handle key
        }
    }
}
```

- Poll with a short timeout (100 ms) to allow periodic redraws even without input.
- Only handle `KeyEventKind::Press` to avoid double-firing on key repeat or release.

---

## Key Patterns Specific to This Project

| Pattern | Where |
|---|---|
| Reconstruct `ListState` each frame from `App` state | `src/ui.rs` all list renderers |
| `block.inner(area)` for manual inner layout | `render_packet_detail`, `render_date_time_filter_popup` |
| Guard `area.width == 0 \|\| area.height == 0` before rendering sub-panes | `render_sub_pane`, `render_tcp_flags_sub_pane` |
| `frame.set_cursor_position` for text cursor in date-time popup | `render_date_time_filter_popup` |
| `terminal.show_cursor()` / `hide_cursor()` outside the draw closure | `sync_date_time_cursor_mode` in `src/main.rs` |
| `title_bottom(footer_line)` for popup footer hints | filter popup, keybindings popup |

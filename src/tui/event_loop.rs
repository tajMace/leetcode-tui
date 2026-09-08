// the polling cycle for the tui

use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph},
};

use crate::{
    cache::save_cached_problem_list,
    client::LeetCodeClient,
    error::Result,
    models::{Difficulty, ProblemSummary, PulledLanguages},
    tui::app::{App, Mode},
};

pub const PAGE_SIZE: i32 = 100;

pub fn run(problems: Vec<ProblemSummary>, pulled: PulledLanguages) -> Result<()> {
    let mut terminal = ratatui::init();
    let result = run_event_loop(&mut terminal, problems, pulled);
    ratatui::restore();

    result
}

fn run_event_loop(
    terminal: &mut DefaultTerminal,
    problems: Vec<ProblemSummary>,
    pulled: PulledLanguages,
) -> Result<()> {
    let mut app = App::new(problems, pulled);

    while !app.should_quit {
        terminal.draw(|frame| render(frame, &app))?;

        if let Event::Key(key) = event::read()? {
            match app.mode {
                Mode::ProblemList => match key.code {
                    KeyCode::Up => app.select_previous_problem(),
                    KeyCode::Down => app.select_next_problem(),
                    KeyCode::Char('r') => pull_problem_list(terminal, &mut app)?,
                    KeyCode::Char('q') => app.quit(),
                    KeyCode::Enter => app.open_language_selection(),
                    _ => {}
                },
                Mode::LanguageSelect => match key.code {
                    KeyCode::Up => app.select_previous_lang(),
                    KeyCode::Down => app.select_next_lang(),
                    KeyCode::Esc => app.close_language_selection(),
                    KeyCode::Enter => app.pull_selected_problem()?,
                    _ => {}
                },
            }
        }
    }

    Ok(())
}

fn render(frame: &mut Frame, app: &App) {
    render_problem_list(frame, app);
    render_keybind_box(frame, app);
    if app.mode == Mode::LanguageSelect {
        render_language_dropdown(frame, app);
    }
    if let Some((count, total)) = app.fetch_progress {
        render_fetch_progress(frame, count, total);
    }
}

fn render_problem_list(frame: &mut Frame, app: &App) {
    let items: Vec<ListItem> = app
        .problems
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let colour = difficulty_colour(&p.difficulty);

            let style = if i == app.problem_selected {
                Style::new()
                    .fg(Color::White)
                    .bg(colour)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(colour)
            };

            let status_char = if p.solved() {
                "✓"
            } else if app.pulled.is_pulled(&p.id) {
                "●"
            } else if p.paid_only {
                "$"
            } else {
                " "
            };

            let text = format!("[{status_char}] {} - {}", p.id, p.title);
            ListItem::new(text).style(style)
        })
        .collect();

    let list = List::new(items).block(Block::bordered().title("LeetCode Problems"));

    let mut state = ListState::default();
    state.select(Some(app.problem_selected));
    frame.render_stateful_widget(list, frame.area(), &mut state);
}

fn difficulty_colour(difficulty: &Difficulty) -> Color {
    match difficulty {
        Difficulty::Easy => Color::Rgb(88, 168, 88), // muted green
        Difficulty::Medium => Color::Rgb(200, 160, 60), // muted amber/gold
        Difficulty::Hard => Color::Rgb(200, 90, 90), // muted red
    }
}

fn render_keybind_box(frame: &mut Frame, app: &App) {
    let bindings: &[(&str, &str)] = match app.mode {
        Mode::ProblemList => &[
            ("↑/↓", "navigate"),
            ("Enter", "select"),
            ("r", "refresh"),
            ("q", "quit"),
        ],
        Mode::LanguageSelect => &[("↑/↓", "navigate"), ("Enter", "pull"), ("Esc", "cancel")],
    };

    let symbols: &[(&str, &str)] = &[("✓", "solved"), ("●", "pulled"), ("$", "paid only")];

    let mut text: Vec<Line> = bindings
        .iter()
        .map(|(key, action)| {
            Line::from(vec![
                Span::styled(
                    *key,
                    Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!(" → {action}")),
            ])
        })
        .collect();

    text.push(Line::raw("")); // blank separator line
    text.extend(symbols.iter().map(|(symbol, meaning)| {
        Line::from(vec![Span::raw(format!("{symbol} ")), Span::raw(*meaning)])
    }));

    let area = keybind_box_area(frame.area(), text.len() as u16);
    frame.render_widget(ratatui::widgets::Clear, area);
    frame.render_widget(
        Paragraph::new(text).block(Block::bordered().title("Keys")),
        area,
    );
}

fn keybind_box_area(full_area: Rect, content_lines: u16) -> Rect {
    let width = 20;
    let height = content_lines + 2;
    Rect {
        x: full_area.width.saturating_sub(width), // right-aligned
        y: 0,                                     // top-aligned
        width: width.min(full_area.width),
        height: height.min(full_area.height),
    }
}

fn render_language_dropdown(frame: &mut Frame, app: &App) {
    let selected_id = &app.selected_problem().id;
    let items: Vec<ListItem> = app
        .lang_options
        .iter()
        .map(|lang| {
            let pulled_marker = if app.pulled.is_pulled_in_language(selected_id, *lang) {
                " ● "
            } else {
                "   "
            };
            ListItem::new(format!("{pulled_marker}{}", lang.as_str()))
        })
        .collect();

    let list = List::new(items)
        .block(Block::bordered().title("Select Language"))
        .highlight_style(Style::new().reversed());

    let mut state = ListState::default();
    state.select(Some(app.lang_selected));

    let area = centered_rect(40, 60, frame.area());
    frame.render_widget(ratatui::widgets::Clear, area);
    frame.render_stateful_widget(list, area, &mut state);
}

/// Carves out a smaller, centered rectangle from the given area - the
/// standard `ratatui` pattern for popups/dropdowns. `percent_x`/`percent_y`
/// control how much of the screen the popup occupies (e.g. 40 = 40%).
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let [area] = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .areas(area);
    area
}

fn pull_problem_list(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
    let mut all_problems = Vec::new();
    let mut skip = 0;

    let client = LeetCodeClient::new()?;

    loop {
        let (page, total) = client.fetch_problem_page(skip, PAGE_SIZE)?;
        all_problems.extend(page);

        app.fetch_progress = Some((all_problems.len(), total as usize));
        terminal.draw(|frame| render(frame, app))?;

        if all_problems.len() >= total {
            break;
        }
        skip += PAGE_SIZE;
    }

    app.fetch_progress = None;
    save_cached_problem_list(&all_problems)?;
    app.problems = all_problems;

    Ok(())
}

fn render_fetch_progress(frame: &mut Frame, count: usize, total: usize) {
    let percent = if total > 0 { (count * 100) / total } else { 0 };
    let filled = (percent / 5) as usize;
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(20 - filled));

    let area = centered_rect(40, 20, frame.area());

    let content_lines = 2; // "Fetching..." line + bar line
    let inner_height = area.height.saturating_sub(2); // minus top/bottom border
    let top_padding = inner_height.saturating_sub(content_lines).saturating_div(2);

    let mut lines: Vec<Line> = (0..top_padding).map(|_| Line::raw("")).collect();
    lines.push(Line::raw(format!("Fetching problems... {count}/{total}")));
    lines.push(Line::raw(format!("{bar} {percent}%")));

    frame.render_widget(ratatui::widgets::Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::bordered().title("Refreshing"))
            .alignment(Alignment::Center),
        area,
    );
}

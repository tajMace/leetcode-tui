// the polling cycle for the tui

use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, List, ListItem, ListState},
};

use crate::{
    error::Result,
    models::{Difficulty, ProblemSummary, PulledLanguages},
    tui::app::{App, Mode},
};

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
                    KeyCode::Char('r') => app.pull_problem_list()?,
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
    if app.mode == Mode::LanguageSelect {
        render_language_dropdown(frame, app);
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

// ratatui app runner

use clap::ValueEnum;

use crate::{
    client::LeetCodeClient,
    commands,
    error::Result,
    models::{LangSlug, ProblemSummary, PulledLanguages},
    tui::app::Mode::ProblemList,
};

#[derive(PartialEq)]
pub enum Mode {
    ProblemList,
    LanguageSelect,
}

pub struct App {
    // problem (meta)data
    pub problems: Vec<ProblemSummary>,
    pub pulled: PulledLanguages,
    pub problem_selected: usize,
    pub should_quit: bool,

    // lang dropdown (meta)data
    pub mode: Mode,
    pub lang_options: Vec<LangSlug>,
    pub lang_selected: usize,

    // in action status -> x out of y
    pub fetch_progress: Option<(usize, usize)>,
}

impl App {
    pub fn new(problems: Vec<ProblemSummary>, pulled: PulledLanguages) -> Self {
        Self {
            problems,
            pulled,
            problem_selected: 0,
            should_quit: false,

            mode: ProblemList,
            lang_options: LangSlug::value_variants().to_vec(),
            lang_selected: 0,

            fetch_progress: None,
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /* ===== PROBLEM MENU ===== */
    pub fn select_next_problem(&mut self) {
        self.problem_selected = (self.problem_selected + 1) % self.problems.len();
    }

    pub fn select_previous_problem(&mut self) {
        self.problem_selected =
            (self.problem_selected + self.problems.len() - 1) % self.problems.len();
    }

    pub fn pull_selected_problem(&mut self) -> Result<()> {
        let slug = self.problems[self.problem_selected].title_slug.clone();
        let id = self.problems[self.problem_selected].id.clone();
        let lang = self.lang_options[self.lang_selected];

        commands::pull(slug, lang)?;
        self.pulled.mark_pulled(&id, lang);
        self.close_language_selection();

        Ok(())
    }

    // pub fn pull_problem_list(&mut self) -> Result<()> {
    //     cache::download_and_save_problem_list()?;
    //     self.problems = load_cached_problem_list()?;
    //     self.pulled = backfill_pulled_from_disk(&self.problems)?;

    //     Ok(())
    // }

    pub fn pull_daily_challenge(&mut self) -> Result<()> {
        let client = LeetCodeClient::new()?;
        let challenge_id = client.fetch_daily_challenge_id()?;
        self.problem_selected = challenge_id as usize;
        self.open_language_selection();

        Ok(())
    }

    /* ===== LANG MENU ===== */
    pub fn open_language_selection(&mut self) {
        self.mode = Mode::LanguageSelect;
    }

    pub fn select_next_lang(&mut self) {
        self.lang_selected = (self.lang_selected + 1) % self.lang_options.len();
    }

    pub fn select_previous_lang(&mut self) {
        self.lang_selected =
            (self.lang_selected + self.lang_options.len() - 1) % self.lang_options.len();
    }

    pub fn close_language_selection(&mut self) {
        self.mode = Mode::ProblemList;
    }

    /* ===== pub helpers ===== */
    pub fn selected_problem(&self) -> &ProblemSummary {
        &self.problems[self.problem_selected]
    }
}

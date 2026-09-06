// thin wrapper for calling the problem list frontend

use crate::{cache, error::Result, tui};

/// loads the ratatui instance containing a list of LeetCode problems
pub fn list() -> Result<()> {
    let problems = cache::load_cached_problem_list()?;
    let pulled = cache::load_pulled_languages()?;
    tui::run(problems, pulled)
}

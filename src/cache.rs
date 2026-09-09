// cache related helper functionality
use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Serialize, de::DeserializeOwned};

use crate::{
    client::LeetCodeClient,
    error::{LeetCodeError, Result},
    models::{ProblemSummary, PulledLanguages},
};

/*
 * ========== Problem List Cache ==========
 */
pub fn load_cached_problem_list() -> Result<Vec<ProblemSummary>> {
    load_from(&get_or_create_problem_cache_filepath()?)
}

pub fn save_cached_problem_list(problems: &[ProblemSummary]) -> Result<()> {
    save_to(&get_or_create_problem_cache_filepath()?, &problems)
}

pub fn download_and_save_problem_list() -> Result<()> {
    let client = LeetCodeClient::new()?;
    let problems = client.fetch_problem_list()?;
    save_cached_problem_list(&problems)
}

/*
 * ========== Pulled Languages Tracking ==========
 */
pub fn load_pulled_languages() -> Result<PulledLanguages> {
    load_from(&get_or_create_pulled_cache_filepath()?)
}

pub fn save_pulled_languages(languages: &PulledLanguages) -> Result<()> {
    save_to(&get_or_create_pulled_cache_filepath()?, languages)
}

// pub fn backfill_pulled_from_disk(problems: &[ProblemSummary]) -> Result<PulledLanguages> {
//     let mut pulled = load_pulled_languages()?;

//     for problem in problems {
//         for lang in LangSlug::value_variants() {
//             let filepath = get_challenge_filepath(&problem.title_slug, *lang)?;
//             if filepath.exists() {
//                 pulled.mark_pulled(&problem.id, *lang);
//             }
//         }
//     }

//     save_pulled_languages(&pulled)?;

//     Ok(pulled)
// }

/*
 * ========== HELPERS ==========
 */

/* generic loading and saving helpers */
fn load_from<T: DeserializeOwned + Default>(path: &Path) -> Result<T> {
    if !path.exists() {
        return Ok(T::default());
    }
    let contents = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&contents)?)
}

fn save_to<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string(value)?)?;
    Ok(())
}

/* cache filepath helpers */
fn get_or_create_problem_cache_filepath() -> Result<PathBuf> {
    get_or_create_from_cache_file("problems.json")
}

fn get_or_create_pulled_cache_filepath() -> Result<PathBuf> {
    get_or_create_from_cache_file("pulled.json")
}

fn get_or_create_from_cache_file(file: &str) -> Result<PathBuf> {
    let path = get_from_cache_file(file)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(path)
}

fn get_from_cache_file(file: &str) -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir().ok_or_else(|| LeetCodeError::CacheDir)?;
    Ok(cache_dir.join("lc-cli").join(file))
}

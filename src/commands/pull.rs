// fetch a problem, write its starter code to src/problems/<slug>/q.<fe>

use std::fs;

use crate::{
    cache::{load_pulled_languages, save_pulled_languages},
    client::LeetCodeClient,
    commands::solution_file::{
        generate_problem_file, generate_spec_file, get_challenge_dir, get_challenge_filepath,
        get_spec_filepath,
    },
    error::Result,
    manifest::add_bin_entry,
    models::{LangSlug, Problem},
};

pub fn pull(slug: String, lang: LangSlug) -> Result<()> {
    let challenge_filepath = get_challenge_filepath(&slug, lang)?;

    // don't repull existing challenge
    /* REPLACE WITH CACHE CHECK */
    if fs::exists(&challenge_filepath)? {
        return Ok(());
    };

    let client = LeetCodeClient::new()?;
    let problem = client.fetch_problem(&slug)?;

    save_problem_to_disk(&slug, lang, &problem)?;

    Ok(())
}

fn save_problem_to_disk(slug: &str, lang: LangSlug, problem: &Problem) -> Result<()> {
    let dirpath = get_challenge_dir(&slug)?;

    fs::create_dir_all(&dirpath)?;
    write_spec_if_missing(slug, problem)?;
    write_problem_to_store(slug, lang, problem)?;
    write_problem_to_pulled(lang, problem)?;

    Ok(())
}

fn write_spec_if_missing(slug: &str, problem: &Problem) -> Result<()> {
    let spec_filepath = get_spec_filepath(&slug)?;
    if !fs::exists(&spec_filepath)? {
        fs::write(&spec_filepath, generate_spec_file(&problem)?)?;
    }

    Ok(())
}

fn write_problem_to_store(slug: &str, lang: LangSlug, problem: &Problem) -> Result<()> {
    let challenge_filepath = get_challenge_filepath(&slug, lang)?;

    fs::write(&challenge_filepath, generate_problem_file(&problem, lang)?)?;
    if lang == LangSlug::Rust {
        add_bin_entry(&slug)?;
    }

    Ok(())
}

fn write_problem_to_pulled(lang: LangSlug, problem: &Problem) -> Result<()> {
    let mut pulled = load_pulled_languages()?;
    pulled.mark_pulled(&problem.question_frontend_id, lang);
    save_pulled_languages(&pulled)?;

    Ok(())
}

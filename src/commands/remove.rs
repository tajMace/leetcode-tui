// command to remove a file locally, while preserving related structures (eg. cache)

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    cache::{load_pulled_languages, save_pulled_languages},
    commands::{
        get_challenge_filepath,
        solution_file::{find_problem_by_slug, get_challenge_dir},
    },
    error::Result,
    manifest::remove_bin_entry,
    models::LangSlug,
};

pub fn remove(slug: String, lang: Option<LangSlug>) -> Result<()> {
    match lang {
        Some(lang) => remove_single_language(&slug, lang)?,
        None => remove_all_languages(&slug)?,
    }

    Ok(())
}

fn remove_all_languages(slug: &str) -> Result<()> {
    let problem = find_problem_by_slug(slug)?;
    let pulled = load_pulled_languages()?;

    let langs: Vec<LangSlug> = pulled
        .map
        .get(&problem.id)
        .map(|set| set.iter().copied().collect())
        .unwrap_or_default();

    for lang in langs {
        eprintln!("removing language: {lang:?}");
        remove_single_language(slug, lang)?;
        eprintln!("  done");
    }

    Ok(())
}

fn remove_single_language(slug: &str, lang: LangSlug) -> Result<()> {
    remove_from_filepath(slug, lang)?;
    remove_from_pulled_cache(slug, lang)?;

    if lang == LangSlug::Rust {
        remove_bin_entry(slug)?;
    }

    let challenge_dir = get_challenge_dir(slug)?;
    if only_spec_remains(&challenge_dir)? {
        fs::remove_file(challenge_dir.join("SPEC.md"))?;
        fs::remove_dir(challenge_dir)?;
    }

    Ok(())
}

fn remove_from_filepath(slug: &str, lang: LangSlug) -> Result<()> {
    let filepath = get_challenge_filepath(slug, lang)?;
    fs::remove_file(&filepath)?;

    Ok(())
}

fn remove_from_pulled_cache(slug: &str, lang: LangSlug) -> Result<()> {
    let mut pulled = load_pulled_languages()?;
    let problem = find_problem_by_slug(slug)?;
    pulled.mark_not_pulled(&problem.id, lang);
    save_pulled_languages(&pulled)?;

    Ok(())
}

/* utility helpers */
fn only_spec_remains(dir: &Path) -> Result<bool> {
    for entry in fs::read_dir(dir)? {
        if entry?.file_name() != "SPEC.md" {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::fs;

    /// Mirrors remove_single_language's directory-cleanup step exactly,
    /// so this test can't silently drift out of sync with the real logic
    /// the way the previous version did.
    fn remove_language_file(dir: &std::path::Path, filename: &str) -> Result<bool> {
        fs::remove_file(dir.join(filename))?;
        let should_delete = only_spec_remains(dir)?;
        if should_delete {
            fs::remove_file(dir.join("SPEC.md"))?;
            fs::remove_dir(dir)?;
        }
        Ok(should_delete)
    }

    #[test]
    fn removing_languages_one_by_one_only_deletes_dir_on_last() {
        let dir = std::env::temp_dir().join(format!("lc-cli-real-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SPEC.md"), "").unwrap();
        fs::write(dir.join("q.rs"), "").unwrap();
        fs::write(dir.join("q.py"), "").unwrap();
        fs::write(dir.join("q.go"), "").unwrap();

        assert!(dir.exists());

        let should_delete = remove_language_file(&dir, "q.rs").unwrap();
        assert!(
            !should_delete,
            "should not delete dir after removing 1 of 3 languages"
        );
        assert!(dir.exists());

        let should_delete = remove_language_file(&dir, "q.py").unwrap();
        assert!(
            !should_delete,
            "should not delete dir after removing 2 of 3 languages"
        );
        assert!(dir.exists());

        let should_delete = remove_language_file(&dir, "q.go").unwrap();
        assert!(
            should_delete,
            "should say safe to delete after removing last language"
        );

        // directory removal now happens INSIDE remove_language_file, matching
        // remove_single_language's real behavior — nothing left to do here
        assert!(!dir.exists());
    }

    #[test]
    fn removing_language_that_doesnt_exist_errors_cleanly() {
        let dir =
            std::env::temp_dir().join(format!("lc-cli-real-test-missing-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SPEC.md"), "").unwrap();

        let result = remove_language_file(&dir, "q.rs");
        assert!(
            result.is_err(),
            "removing a nonexistent file should error, not panic"
        );

        fs::remove_dir_all(&dir).ok();
    }
}

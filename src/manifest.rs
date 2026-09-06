// provides an API to add [[bin]] entries to the Cargo.toml
// allows for Rust verification in the IDE, without being in a /bin

use crate::{commands::get_challenge_filepath, config::Config, error::Result, models::LangSlug};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn add_bin_entry(slug: &str) -> Result<()> {
    let file_stem = slug.replace('-', "_");
    let filepath = get_challenge_filepath(slug, LangSlug::Rust)?;
    let mut doc: toml::Table = get_cargo_toml_contents()?;
    let bin_array = get_or_create_bin_array(&mut doc);

    if !bin_entry_exists(bin_array, &file_stem) {
        insert_to_bin_array(bin_array, &file_stem, &filepath);
        write_cargo_toml_contents(&doc)?;
    }

    Ok(())
}

/* ========== LOCAL HELPERS ========== */

fn get_cargo_toml_contents() -> Result<toml::Table> {
    let config = Config::load()?;
    let config_file = config.require_storage_dir()?.join("Cargo.toml");
    let contents = fs::read_to_string(config_file)?;
    Ok(toml::from_str(&contents)?)
}

fn get_or_create_bin_array(doc: &mut toml::Table) -> &mut Vec<toml::Value> {
    doc.entry("bin")
        .or_insert_with(|| toml::Value::Array(Vec::new()))
        .as_array_mut()
        .expect("bin should be an array")
}

fn bin_entry_exists(bin_array: &[toml::Value], file_stem: &str) -> bool {
    bin_array
        .iter()
        .any(|entry| entry.get("name").and_then(|n| n.as_str()) == Some(file_stem))
}

fn insert_to_bin_array(bin_array: &mut Vec<toml::Value>, file_stem: &str, filepath: &Path) {
    let mut new_entry = toml::Table::new();
    new_entry.insert(
        "name".to_string(),
        toml::Value::String(file_stem.to_string()),
    );
    new_entry.insert(
        "path".to_string(),
        toml::Value::String(filepath.to_string_lossy().to_string()),
    );
    bin_array.push(toml::Value::Table(new_entry));
}

fn write_cargo_toml_contents(doc: &toml::Table) -> Result<()> {
    let config = Config::load()?;
    let config_file = config.require_storage_dir()?.join("Cargo.toml");
    Ok(fs::write(config_file, toml::to_string(&doc)?)?)
}

/*
 * =========== UNIT TESTS ==========
 */
#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(name: &str, path: &str) -> toml::Value {
        let mut table = toml::Table::new();
        table.insert("name".to_string(), toml::Value::String(name.to_string()));
        table.insert("path".to_string(), toml::Value::String(path.to_string()));
        toml::Value::Table(table)
    }

    // --- bin_entry_exists ---

    #[test]
    fn bin_entry_exists_finds_matching_name() {
        let entries = vec![
            sample_entry("two_sum", "src/problems/two-sum/q.rs"),
            sample_entry("valid_parentheses", "src/problems/valid-parentheses/q.rs"),
        ];
        assert!(bin_entry_exists(&entries, "two_sum"));
    }

    #[test]
    fn bin_entry_exists_returns_false_when_absent() {
        let entries = vec![sample_entry("two_sum", "src/problems/two-sum/q.rs")];
        assert!(!bin_entry_exists(&entries, "three_sum"));
    }

    #[test]
    fn bin_entry_exists_returns_false_on_empty_array() {
        let entries: Vec<toml::Value> = vec![];
        assert!(!bin_entry_exists(&entries, "anything"));
    }

    #[test]
    fn bin_entry_exists_ignores_entries_missing_name_field() {
        // malformed entry with no "name" key at all — shouldn't panic, just not match
        let mut malformed = toml::Table::new();
        malformed.insert(
            "path".to_string(),
            toml::Value::String("somewhere.rs".to_string()),
        );
        let entries = vec![toml::Value::Table(malformed)];
        assert!(!bin_entry_exists(&entries, "two_sum"));
    }

    // --- insert_to_bin_array ---

    #[test]
    fn insert_to_bin_array_adds_correct_name_and_path() {
        let mut entries: Vec<toml::Value> = vec![];
        insert_to_bin_array(&mut entries, "two_sum", Path::new("1-two-sum"));

        assert_eq!(entries.len(), 1);
        let name = entries[0].get("name").and_then(|v| v.as_str());
        let path = entries[0].get("path").and_then(|v| v.as_str());
        assert_eq!(name, Some("two_sum"));
        assert_eq!(path, Some("src/problems/two-sum/q.rs"));
    }

    #[test]
    fn insert_to_bin_array_appends_without_removing_existing() {
        let mut entries = vec![sample_entry("existing", "src/problems/existing/q.rs")];
        insert_to_bin_array(&mut entries, "two_sum", Path::new("1-two-sum"));

        assert_eq!(entries.len(), 2);
        assert!(bin_entry_exists(&entries, "existing"));
        assert!(bin_entry_exists(&entries, "two_sum"));
    }

    // --- get_bin_entries ---

    #[test]
    fn get_bin_entries_creates_array_when_missing() {
        let mut doc = toml::Table::new();
        let array = get_or_create_bin_array(&mut doc);
        assert!(array.is_empty());
        // confirm it actually got inserted into doc, not just handed back empty
        assert!(doc.contains_key("bin"));
    }

    #[test]
    fn get_bin_entries_returns_existing_array_unmodified() {
        let mut doc = toml::Table::new();
        doc.insert(
            "bin".to_string(),
            toml::Value::Array(vec![sample_entry(
                "existing",
                "src/problems/existing/rust.rs",
            )]),
        );
        let array = get_or_create_bin_array(&mut doc);
        assert_eq!(array.len(), 1);
    }

    #[test]
    #[should_panic(expected = "bin should be an array")]
    fn get_bin_entries_panics_if_bin_key_is_not_an_array() {
        let mut doc = toml::Table::new();
        doc.insert(
            "bin".to_string(),
            toml::Value::String("not an array".to_string()),
        );
        get_or_create_bin_array(&mut doc); // should hit the .expect() and panic
    }
}

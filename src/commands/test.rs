// run local src/problems/<slug>/q.<fe> solution against the example testcases
use crate::{
    client::LeetCodeClient,
    commands::solution_file::read_and_parse_solution_file,
    error::Result,
    models::{LangSlug, RunResult},
};

pub fn test(slug: String, lang: LangSlug) -> Result<()> {
    let solution = read_and_parse_solution_file(&slug, lang)?;

    let client = LeetCodeClient::new()?;
    let question = client.fetch_problem(&slug)?;
    let result = client.run_testcases(&question, &solution)?;

    print_run_result(&result);

    Ok(())
}

/*
 * ========== UI STUFF ==========
 */

const RESET: &str = "\x1B[0m";
const BOLD: &str = "\x1B[1m";
const GREEN: &str = "\x1B[1;32m";
const RED: &str = "\x1B[1;31m";
const DIM: &str = "\x1B[2m";
const CLEAR_SCREEN: &str = "\x1B[2J\x1B[1;1H";

/* ---------- test (RunResult) ---------- */

pub fn print_run_result(result: &RunResult) {
    print!("{CLEAR_SCREEN}");

    if let Some(full_compile_error) = &result.full_compile_error {
        print_banner("✗  COMPILE ERROR", RED);
        println!();
        println!("{full_compile_error}");
        return;
    }

    if result.correct_answer.unwrap_or(false) {
        print_banner("✓  ACCEPTED", GREEN);
        println!();
        println!(
            "  {}/{} testcases passed",
            result.total_correct.unwrap_or(0),
            result.total_testcases.unwrap_or(0),
        );
        println!(
            "  Runtime: {}  ({})",
            result.status_runtime,
            percentile_str(result.runtime_percentile, "faster")
        );
        println!(
            "  Memory:  {}  ({})",
            result.status_memory,
            percentile_str(result.memory_percentile, "less memory")
        );
        return;
    }

    print_banner("✗  FAILED", RED);
    println!();
    println!(
        "  {}/{} testcases passed",
        result.total_correct.unwrap_or(0),
        result.total_testcases.unwrap_or(0),
    );
    println!();

    let compare_result = result.compare_result.as_deref().unwrap_or("");
    for (i, passed) in compare_result.chars().enumerate() {
        if passed == '0' {
            let got = result
                .code_answer
                .as_ref()
                .and_then(|v| v.get(i))
                .map(String::as_str)
                .unwrap_or("?");
            let expected = result
                .expected_code_answer
                .as_ref()
                .and_then(|v| v.get(i))
                .map(String::as_str)
                .unwrap_or("?");
            print_testcase_diff(i, expected, got);
        }
    }

    print_stdout(result);
}

/* ---------- shared helpers ---------- */

fn print_banner(label: &str, color: &str) {
    let width = 41;
    let padding = (width - 2 - label.chars().count()) / 2;
    println!("{color}{BOLD}╔{}╗{RESET}", "═".repeat(width));
    println!(
        "{color}{BOLD}║{}{label}{}║{RESET}",
        " ".repeat(padding),
        " ".repeat(width - padding - label.chars().count()),
    );
    println!("{color}{BOLD}╚{}╝{RESET}", "═".repeat(width));
}

fn percentile_str(percentile: Option<f32>, comparison: &str) -> String {
    match percentile {
        Some(p) => format!("beats {p:.1}% of submissions on {comparison}"),
        None => "percentile unavailable".to_string(),
    }
}

fn print_testcase_diff(index: usize, expected: &str, got: &str) {
    println!("{DIM}  testcase {}:{RESET}", index + 1);
    println!("    expected: {expected}");
    println!("    got:      {got}");
}

fn print_stdout(result: &RunResult) {
    // filter out empty stdout
    let Some(outputs) = &result.std_output_list else {
        return;
    };
    if !outputs.iter().any(|s| !s.is_empty()) {
        return;
    }

    println!();
    println!("  stdout:");
    for (i, output) in outputs.iter().enumerate() {
        if !output.is_empty() {
            println!("    testcase {}: {output}", i + 1);
        }
    }
}

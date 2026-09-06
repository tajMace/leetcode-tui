// submit local src/bin/<slug>.rs solution for real judging

use crate::{
    client::LeetCodeClient,
    commands::solution_file::read_and_parse_solution_file,
    error::Result,
    models::{LangSlug, SubmissionResult},
};

pub fn submit(slug: String, lang: LangSlug) -> Result<()> {
    let solution = read_and_parse_solution_file(&slug, lang)?;

    let client = LeetCodeClient::new()?;
    let question = client.fetch_problem(&slug)?;
    let result = client.submit_solution(&question, &solution)?;

    print_submission_result(&result);

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

/* ---------- submit (SubmissionResult) ---------- */

pub fn print_submission_result(result: &SubmissionResult) {
    print!("{CLEAR_SCREEN}");

    match result {
        SubmissionResult::Accepted {
            runtime_percentile,
            memory_percentile,
            total_correct,
            total_testcases,
        } => {
            println!("{GREEN}{BOLD}╔═══════════════════════════════════════╗{RESET}");
            println!("{GREEN}{BOLD}║              ✓  ACCEPTED              ║{RESET}");
            println!("{GREEN}{BOLD}╚═══════════════════════════════════════╝{RESET}");
            println!();
            println!("{DIM}────────────────────────────────────────{RESET}");
            println!("  {BOLD}Testcases{RESET}  {total_correct}/{total_testcases} passed");
            println!(
                "  {BOLD}Runtime{RESET}    {}  beats {GREEN}{:.1}%{RESET} of submissions",
                percentile_bar(*runtime_percentile),
                runtime_percentile
            );
            println!(
                "  {BOLD}Memory{RESET}     {}  beats {GREEN}{:.1}%{RESET} of submissions",
                percentile_bar(*memory_percentile),
                memory_percentile
            );
            println!("{DIM}────────────────────────────────────────{RESET}");
        }
        SubmissionResult::WrongAnswer {
            last_testcase,
            expected_output,
            code_output,
            total_correct,
            total_testcases,
            ..
        } => {
            println!("{RED}{BOLD}╔═══════════════════════════════════════╗{RESET}");
            println!("{RED}{BOLD}║            ✗  WRONG ANSWER            ║{RESET}");
            println!("{RED}{BOLD}╚═══════════════════════════════════════╝{RESET}");
            println!();
            println!("  {total_correct}/{total_testcases} testcases passed");
            println!();
            println!("    input:    {last_testcase}");
            println!("    expected: {expected_output}");
            println!("    got:      {code_output}");
        }
        SubmissionResult::CompileError {
            full_compile_error, ..
        } => {
            println!("{RED}{BOLD}╔═══════════════════════════════════════╗{RESET}");
            println!("{RED}{BOLD}║           ✗  COMPILE ERROR            ║{RESET}");
            println!("{RED}{BOLD}╚═══════════════════════════════════════╝{RESET}");
            println!();
            println!("{full_compile_error}");
        }
        SubmissionResult::RuntimeError {
            full_runtime_error, ..
        } => {
            println!("{RED}{BOLD}╔═══════════════════════════════════════╗{RESET}");
            println!("{RED}{BOLD}║           ✗  RUNTIME ERROR            ║{RESET}");
            println!("{RED}{BOLD}╚═══════════════════════════════════════╝{RESET}");
            println!();
            println!("{full_runtime_error}");
        }
        SubmissionResult::TimeLimitExceeded { .. } => {
            println!("{RED}{BOLD}╔═══════════════════════════════════════╗{RESET}");
            println!("{RED}{BOLD}║         ✗  TIME LIMIT EXCEEDED        ║{RESET}");
            println!("{RED}{BOLD}╚═══════════════════════════════════════╝{RESET}");
        }
        SubmissionResult::MemoryLimitExceeded { .. } => {
            println!("{RED}{BOLD}╔═══════════════════════════════════════╗{RESET}");
            println!("{RED}{BOLD}║        ✗  MEMORY LIMIT EXCEEDED       ║{RESET}");
            println!("{RED}{BOLD}╚═══════════════════════════════════════╝{RESET}");
        }
        SubmissionResult::OutputLimitExceeded { .. } => {
            println!("{RED}{BOLD}╔═══════════════════════════════════════╗{RESET}");
            println!("{RED}{BOLD}║        ✗  OUTPUT LIMIT EXCEEDED       ║{RESET}");
            println!("{RED}{BOLD}╚═══════════════════════════════════════╝{RESET}");
        }
        SubmissionResult::Unknown(code, msg) => {
            println!("{RED}{BOLD}╔═══════════════════════════════════════╗{RESET}");
            println!("{RED}{BOLD}║           ?  UNKNOWN OUTCOME          ║{RESET}");
            println!("{RED}{BOLD}╚═══════════════════════════════════════╝{RESET}");
            println!();
            println!("  status_code {code}: {msg}");
        }
    }
}

fn percentile_bar(pct: f32) -> String {
    let filled = (pct / 5.0).round().clamp(0.0, 20.0) as usize; // 20 segments = 5% each
    format!(
        "{GREEN}{}{DIM}{}{RESET}",
        "█".repeat(filled),
        "░".repeat(20 - filled)
    )
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

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
            std_output,
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
            print_single_stdout(std_output);
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
            full_runtime_error,
            std_output,
            ..
        } => {
            println!("{RED}{BOLD}╔═══════════════════════════════════════╗{RESET}");
            println!("{RED}{BOLD}║           ✗  RUNTIME ERROR            ║{RESET}");
            println!("{RED}{BOLD}╚═══════════════════════════════════════╝{RESET}");
            println!();
            println!("{full_runtime_error}");
            print_single_stdout(std_output);
        }
        SubmissionResult::TimeLimitExceeded { std_output, .. } => {
            println!("{RED}{BOLD}╔═══════════════════════════════════════╗{RESET}");
            println!("{RED}{BOLD}║         ✗  TIME LIMIT EXCEEDED        ║{RESET}");
            println!("{RED}{BOLD}╚═══════════════════════════════════════╝{RESET}");
            print_single_stdout(std_output);
        }
        SubmissionResult::MemoryLimitExceeded { std_output, .. } => {
            println!("{RED}{BOLD}╔═══════════════════════════════════════╗{RESET}");
            println!("{RED}{BOLD}║        ✗  MEMORY LIMIT EXCEEDED       ║{RESET}");
            println!("{RED}{BOLD}╚═══════════════════════════════════════╝{RESET}");
            print_single_stdout(std_output);
        }
        SubmissionResult::OutputLimitExceeded { std_output, .. } => {
            println!("{RED}{BOLD}╔═══════════════════════════════════════╗{RESET}");
            println!("{RED}{BOLD}║        ✗  OUTPUT LIMIT EXCEEDED       ║{RESET}");
            println!("{RED}{BOLD}╚═══════════════════════════════════════╝{RESET}");
            print_single_stdout(std_output);
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

fn print_single_stdout(std_output: &Option<String>) {
    let Some(output) = std_output else {
        return;
    };
    if output.is_empty() {
        return;
    }
    println!();
    println!("  stdout: {output}");
}

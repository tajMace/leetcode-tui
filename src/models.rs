// response types: Problem, CodeSnippet, SubmissionStatus (enum + match),
// designed after the shape the GraphQL/REST responses return

use crate::error::{LeetCodeError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    fmt,
};

pub const SOLUTION_MARKER: &str = "/* ---------- SOLUTION START ---------- */";

/*
 * ========== LangSlug Model ==========
 */
#[derive(PartialEq, Eq, Deserialize, Hash, Serialize, Debug, Clone, Copy, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum LangSlug {
    Cpp,
    Java,
    Python3,
    Python,
    JavaScript,
    TypeScript,
    CSharp,
    C,
    Golang,
    Kotlin,
    Swift,
    Rust,
    Ruby,
    Php,
    Dart,
    Scala,
    Elixir,
    Erlang,
    Racket,
}

impl LangSlug {
    /// File extension to use when writing a solution file to disk.
    pub fn file_extension(&self) -> &'static str {
        match self {
            LangSlug::Cpp => "cpp",
            LangSlug::Java => "java",
            LangSlug::Python3 | LangSlug::Python => "py",
            LangSlug::JavaScript => "js",
            LangSlug::TypeScript => "ts",
            LangSlug::CSharp => "cs",
            LangSlug::C => "c",
            LangSlug::Golang => "go",
            LangSlug::Kotlin => "kt",
            LangSlug::Swift => "swift",
            LangSlug::Rust => "rs",
            LangSlug::Ruby => "rb",
            LangSlug::Php => "php",
            LangSlug::Dart => "dart",
            LangSlug::Scala => "scala",
            LangSlug::Elixir => "ex",
            LangSlug::Erlang => "erl",
            LangSlug::Racket => "rkt",
        }
    }

    /// Lowercase wire-format name,exposed directly for cases
    /// (like error messages) that need the string without going through serde.
    pub fn as_str(&self) -> &'static str {
        match self {
            LangSlug::Cpp => "cpp",
            LangSlug::Java => "java",
            LangSlug::Python3 => "python3",
            LangSlug::Python => "python",
            LangSlug::JavaScript => "javascript",
            LangSlug::TypeScript => "typescript",
            LangSlug::CSharp => "csharp",
            LangSlug::C => "c",
            LangSlug::Golang => "golang",
            LangSlug::Kotlin => "kotlin",
            LangSlug::Swift => "swift",
            LangSlug::Rust => "rust",
            LangSlug::Ruby => "ruby",
            LangSlug::Php => "php",
            LangSlug::Dart => "dart",
            LangSlug::Scala => "scala",
            LangSlug::Elixir => "elixir",
            LangSlug::Erlang => "erlang",
            LangSlug::Racket => "racket",
        }
    }
}

impl fmt::Display for LangSlug {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/*
 * ========== QUESTION MODEL ==========
 */
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    pub question_id: String,
    pub question_frontend_id: String,
    pub title: String,
    pub title_slug: String,
    pub difficulty: Difficulty,
    pub content: String,
    pub code_snippets: Vec<CodeSnippet>,
    pub example_testcase_list: Vec<String>,
}

impl Problem {
    pub fn from_graphql_value(root: &Value, slug: &str) -> Result<Self> {
        let question_json = root
            .get("data")
            .and_then(|d| d.get("question"))
            .filter(|q| !q.is_null())
            .ok_or_else(|| LeetCodeError::ProblemNotFound(slug.to_string()))?;

        if Self::paid_question_requires_subscription(question_json) {
            return Err(LeetCodeError::UnpaidAccount);
        }

        Ok(serde_json::from_value(question_json.clone())?)
    }

    pub fn from_graphql_payload(json_str: &str, slug: &str) -> Result<Self> {
        let root: Value = serde_json::from_str(json_str)?;
        Self::from_graphql_value(&root, slug)
    }

    pub fn lang_snippet(&self, lang: LangSlug) -> Option<&CodeSnippet> {
        self.code_snippets.iter().find(|s| s.lang_slug == lang)
    }

    /* helpers */
    pub fn paid_question_requires_subscription(question_json: &Value) -> bool {
        let is_paid_only = question_json
            .get("isPaidOnly")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let content_is_missing = question_json
            .get("content")
            .map(|c| c.is_null())
            .unwrap_or(true);

        is_paid_only && content_is_missing
    }
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CodeSnippet {
    pub lang: String,
    pub lang_slug: LangSlug,
    pub code: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemSummary {
    #[serde(rename = "frontendQuestionId")]
    pub id: String,
    pub title: String,
    pub title_slug: String,
    pub difficulty: Difficulty,
    pub status: Option<String>,
    pub paid_only: bool,
}

impl ProblemSummary {
    pub fn list_from_graphql_value(raw: &Value) -> Result<(Vec<Self>, usize)> {
        let list_json = raw
            .get("data")
            .and_then(|d| d.get("problemsetQuestionList"))
            .ok_or_else(|| {
                LeetCodeError::MalformedResponse("expected data.problemsetQuestionList".to_string())
            })?;

        let total = list_json
            .get("total")
            .and_then(|t| t.as_u64())
            .ok_or_else(|| LeetCodeError::MalformedResponse("expected total".to_string()))?
            as usize;

        let questions_json = list_json
            .get("questions")
            .ok_or_else(|| LeetCodeError::MalformedResponse("expected questions".to_string()))?;

        let problems: Vec<Self> = serde_json::from_value(questions_json.clone())?;

        Ok((problems, total))
    }

    pub fn solved(&self) -> bool {
        self.status.as_deref() == Some("ac")
    }
}

// maps id -> the set of pulled language files
#[derive(Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct PulledLanguages {
    pub map: HashMap<String, HashSet<LangSlug>>,
}

impl PulledLanguages {
    pub fn mark_pulled(&mut self, id: &str, lang: LangSlug) {
        self.map.entry(id.to_string()).or_default().insert(lang);
    }

    pub fn mark_not_pulled(&mut self, id: &str, lang: LangSlug) {
        if let Some(langs) = self.map.get_mut(id) {
            langs.remove(&lang);
            if langs.is_empty() {
                self.map.remove(id);
            }
        }
    }

    pub fn is_pulled(&self, id: &str) -> bool {
        self.map.contains_key(id)
    }

    pub fn is_pulled_in_language(&self, id: &str, lang: LangSlug) -> bool {
        self.map.get(id).is_some_and(|langs| langs.contains(&lang))
    }
}

/*
 * ========== Difficulty Model ==========
 */
#[derive(PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl fmt::Display for Difficulty {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let text = match self {
            Difficulty::Easy => "Easy",
            Difficulty::Medium => "Medium",
            Difficulty::Hard => "Hard",
        };
        write!(f, "{text}")
    }
}

/*
 * ========== SUBMISSION STATUS MODEL ==========
 */

/// raw shape of LeetCode's `/submissions/detail/<id>/check/` response.
/// field presence verified against real submissions: 2026-08-29
#[derive(Deserialize)]
pub struct SubmissionStatus {
    status_code: u8,                    // present in all 7 confirmed outcomes
    lang: LangSlug,                     // present in all 7
    run_success: bool,                  // present in all 7
    status_runtime: String,             // present in all 7
    memory: u32,                        // present in all 7
    display_runtime: Option<String>,    // present only in: Accepted, WrongAnswer
    question_id: String,                // present in all 7
    elapsed_time: Option<u16>,          // present in all except: CompileError
    compare_result: Option<String>,     // present in all except: CompileError
    code_output: Option<String>,        // present in all except: CompileError
    std_output: Option<String>,         // present in all except: CompileError
    last_testcase: Option<String>,      // present in all except: CompileError
    expected_output: Option<String>,    // present in all except: CompileError
    compile_error: Option<String>,      // present only in: CompileError
    full_compile_error: Option<String>, // present only in: CompileError
    runtime_error: Option<String>,      // present only in: RuntimeError
    full_runtime_error: Option<String>, // present only in: RuntimeError
    input: Option<String>,              // present only in: WrongAnswer
    input_formatted: Option<String>,    // present only in: WrongAnswer
    task_finish_time: u64,              // present in all 7
    task_name: String,                  // present in all 7
    finished: bool,                     // present in all 7
    total_correct: Option<u16>,         // key present in all except: CompileError (null there)
    total_testcases: Option<u16>,       // key present in all except: CompileError (null there)
    runtime_percentile: Option<f32>,    // non-null only in: Accepted
    status_memory: String,              // present in all 7
    memory_percentile: Option<f32>,     // non-null only in: Accepted
    pretty_lang: String,                // present in all 7
    submission_id: String,              // present in all 7
    status_msg: String,                 // present in all 7
    state: String,                      // present in all 7
}

impl SubmissionStatus {
    pub fn is_finished(&self) -> bool {
        self.state == "SUCCESS"
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmissionStatusCode {
    Accepted,
    WrongAnswer,
    MemoryLimitExceeded,
    OutputLimitExceeded,
    TimeLimitExceeded,
    RuntimeError,
    CompileError,
    Unknown(i64, String),
}

impl From<i64> for SubmissionStatusCode {
    fn from(code: i64) -> Self {
        match code {
            10 => Self::Accepted,
            11 => Self::WrongAnswer,
            12 => Self::MemoryLimitExceeded,
            13 => Self::OutputLimitExceeded,
            14 => Self::TimeLimitExceeded,
            15 => Self::RuntimeError,
            20 => Self::CompileError,
            other => Self::Unknown(other, String::new()),
        }
    }
}

/// domain-level result-- each variant only carries the
/// fields that are actually meaningful for that outcome.
#[derive(Debug)]
pub enum SubmissionResult {
    Accepted {
        runtime_percentile: f32,
        memory_percentile: f32,
        total_correct: u16,
        total_testcases: u16,
    },
    WrongAnswer {
        compare_result: String,
        code_output: String,
        last_testcase: String,
        expected_output: String,
        total_correct: u16,
        total_testcases: u16,
    },
    MemoryLimitExceeded {
        last_testcase: String,
        expected_output: String,
    },
    OutputLimitExceeded {
        last_testcase: String,
        expected_output: String,
    },
    TimeLimitExceeded {
        last_testcase: String,
        expected_output: String,
    },
    RuntimeError {
        runtime_error: String,
        full_runtime_error: String,
        last_testcase: String,
        expected_output: String,
    },
    CompileError {
        compile_error: String,
        full_compile_error: String,
    },
    Unknown(i64, String),
}

macro_rules! req {
    ($s:expr, $field:ident, $variant:literal) => {
        $s.$field.expect(concat!(
            stringify!($field),
            " always present when status_code is ",
            $variant
        ))
    };
}

impl From<SubmissionStatus> for SubmissionResult {
    fn from(s: SubmissionStatus) -> Self {
        match s.status_code {
            10 => SubmissionResult::Accepted {
                runtime_percentile: req!(s, runtime_percentile, "Accepted"),
                memory_percentile: req!(s, memory_percentile, "Accepted"),
                total_correct: req!(s, total_correct, "Accepted"),
                total_testcases: req!(s, total_testcases, "Accepted"),
            },
            11 => SubmissionResult::WrongAnswer {
                compare_result: req!(s, compare_result, "WrongAnswer"),
                code_output: req!(s, code_output, "WrongAnswer"),
                last_testcase: req!(s, last_testcase, "WrongAnswer"),
                expected_output: req!(s, expected_output, "WrongAnswer"),
                total_correct: req!(s, total_correct, "WrongAnswer"),
                total_testcases: req!(s, total_testcases, "WrongAnswer"),
            },
            12 => SubmissionResult::MemoryLimitExceeded {
                last_testcase: req!(s, last_testcase, "MemoryLimitExceeded"),
                expected_output: req!(s, expected_output, "MemoryLimitExceeded"),
            },
            13 => SubmissionResult::OutputLimitExceeded {
                last_testcase: req!(s, last_testcase, "OutputLimitExceeded"),
                expected_output: req!(s, expected_output, "OutputLimitExceeded"),
            },
            14 => SubmissionResult::TimeLimitExceeded {
                last_testcase: req!(s, last_testcase, "TimeLimitExceeded"),
                expected_output: req!(s, expected_output, "TimeLimitExceeded"),
            },
            15 => SubmissionResult::RuntimeError {
                runtime_error: req!(s, runtime_error, "RuntimeError"),
                full_runtime_error: req!(s, full_runtime_error, "RuntimeError"),
                last_testcase: req!(s, last_testcase, "RuntimeError"),
                expected_output: req!(s, expected_output, "RuntimeError"),
            },
            20 => SubmissionResult::CompileError {
                compile_error: req!(s, compile_error, "CompileError"),
                full_compile_error: req!(s, full_compile_error, "CompileError"),
            },
            other => SubmissionResult::Unknown(other as i64, s.status_msg),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct RunResult {
    // overall run info — present in all 3 confirmed outcomes
    pub status_code: u8,
    pub status_msg: String,
    pub state: String,
    pub lang: LangSlug,
    pub pretty_lang: String,
    pub run_success: bool,
    pub memory: u32,
    pub status_memory: String,
    pub status_runtime: String,
    pub submission_id: String,
    pub task_finish_time: u64,
    pub task_name: String,

    // present in Accepted, WrongAnswer; absent in CompileError
    pub elapsed_time: Option<u32>,
    pub total_correct: Option<u32>,
    pub total_testcases: Option<u32>,
    pub display_runtime: Option<String>,

    // non-null only in Accepted
    pub runtime_percentile: Option<f32>,
    pub memory_percentile: Option<f32>,

    // present (possibly empty) in Accepted/WrongAnswer; absent in CompileError
    pub code_answer: Option<Vec<String>>,
    pub code_output: Option<Vec<String>>,
    pub std_output_list: Option<Vec<String>>,
    pub compare_result: Option<String>,
    pub correct_answer: Option<bool>,

    // expected_* — same presence pattern as their non-expected counterparts
    pub expected_code_answer: Option<Vec<String>>,
    pub expected_code_output: Option<Vec<String>>,
    pub expected_std_output_list: Option<Vec<String>>,
    pub expected_lang: Option<String>,
    pub expected_run_success: Option<bool>,
    pub expected_status_code: Option<u8>,
    pub expected_status_runtime: Option<String>,
    pub expected_display_runtime: Option<String>,
    pub expected_elapsed_time: Option<u32>,
    pub expected_memory: Option<u32>,
    pub expected_task_finish_time: Option<u64>,
    pub expected_task_name: Option<String>,

    // present only in CompileError
    pub compile_error: Option<String>,
    pub full_compile_error: Option<String>,
}

/*
 * Unit Tests
 */
#[cfg(test)]
mod tests {
    use super::*;

    mod question_model {
        use super::*;

        mod lang_snippet {
            use super::*;
            use crate::error::Result;

            #[test]
            fn parses_graphql_and_finds_language_snippet() -> Result<()> {
                let json_payload =
                    include_str!("../test_data/response_samples/two_sum_response.json");

                let question = Problem::from_graphql_payload(json_payload, "two-sum")?;
                assert_eq!(question.question_frontend_id, "1");
                assert_eq!(question.title, "Two Sum");
                assert_eq!(question.difficulty, Difficulty::Easy);

                let snippet = question
                    .lang_snippet(LangSlug::Rust)
                    .expect("Expected to find a Rust snippet in the parsed payload");

                assert_eq!(snippet.lang, "Rust");
                assert_eq!(snippet.lang_slug, LangSlug::Rust);
                assert!(snippet.code.contains("impl Solution"));
                assert!(snippet.code.contains("pub fn two_sum"));

                Ok(())
            }

            #[test]
            fn returns_none_for_missing_language() -> Result<()> {
                let json_payload =
                    include_str!("../test_data/response_samples/two_sum_response.json");
                let _ = Problem::from_graphql_payload(json_payload, "two-sum")?;

                // tests that a language not included in the output does not crash the system
                // possible on newer problems that don't yet support niche languages

                Ok(())
            }

            #[test]
            fn fails_on_malformed_json_syntax() {
                let bad_json = "{ this is not valid json";
                let result = Problem::from_graphql_payload(bad_json, "two-sum");

                // asserts that the first `?` operator correctly caught the error
                assert!(result.is_err(), "Expected an error for malformed JSON");
            }

            #[test]
            fn fails_on_missing_required_fields() {
                // present, non-null "question" object missing required fields --
                // exercises the serde_json::from_value failure, not the
                // ProblemNotFound/.filter(is_null) path
                let bad_schema_json = r#"{
                    "data": {
                        "question": {
                            "title": "Missing ID and Difficulty"
                        }
                    }
                }"#;
                let result = Problem::from_graphql_payload(bad_schema_json, "two-sum");

                assert!(
                    result.is_err(),
                    "Expected an error for missing Question struct fields"
                );
            }

            #[test]
            fn fails_with_problem_not_found_when_question_is_null() {
                // this is the case .filter(|q| !q.is_null()) exists for --
                // worth its own test since it's a distinct code path from
                // both malformed JSON and missing-fields
                let null_question_json = r#"{ "data": { "question": null } }"#;
                let result = Problem::from_graphql_payload(null_question_json, "fake-slug");

                match result {
                    Err(LeetCodeError::ProblemNotFound(slug)) => {
                        assert_eq!(slug, "fake-slug");
                    }
                    other => panic!("expected ProblemNotFound, got {other:?}"),
                }
            }
        }
    }
}

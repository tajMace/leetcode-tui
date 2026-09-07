// LeetCodeClient: wraps an HTTP client, talks to leetcode.com/graphql
// (unauthenticated: fetch problem) and the REST run/submit/check endpoints
// (authenticated: needs the session cookie from config.rs)

use crate::commands::ParsedSolution;
use crate::config::Config;
use crate::error::{LeetCodeError, Result};
use crate::models::{Problem, ProblemSummary, RunResult, SubmissionResult, SubmissionStatus};
use std::cell::RefCell;
use std::time::{Duration, Instant};

const LEETCODE_GRAPHQL_ENDPOINT: &str = "https://leetcode.com/graphql/";
const FETCH_PROBLEM_QUERY: &str = "query fetchProblem($titleSlug: String!) {
  question(titleSlug: $titleSlug) {
    questionId
    questionFrontendId
    title
    titleSlug
    difficulty
    content
    codeSnippets { lang langSlug code }
    exampleTestcaseList
  }
}";
const PROBLEM_LIST_QUERY: &str = "query problemsetQuestionList($categorySlug: String, $limit: Int, $skip: Int, $filters: QuestionListFilterInput) {
  problemsetQuestionList: questionList(
    categorySlug: $categorySlug
    limit: $limit
    skip: $skip
    filters: $filters
  ) {
    total: totalNum
    questions: data {
      difficulty
      frontendQuestionId: questionFrontendId
      paidOnly: isPaidOnly
      status
      title
      titleSlug
    }
  }
}";

pub struct LeetCodeClient {
    http: reqwest::blocking::Client,
    config: Config,
    last_request: RefCell<Instant>,
}

impl LeetCodeClient {
    pub fn new() -> Result<LeetCodeClient> {
        Ok(LeetCodeClient {
            http: reqwest::blocking::Client::new(),
            config: Config::load()?,
            last_request: RefCell::new(Instant::now()),
        })
    }

    /* HTTP FETCH HELPERS */
    fn query_graphql(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.rate_limit();

        let body = serde_json::json!({
            "query": query,
            "variables": variables,
        });

        Ok(self
            .with_auth_headers(
                self.http.post(LEETCODE_GRAPHQL_ENDPOINT),
                LEETCODE_GRAPHQL_ENDPOINT,
            )?
            .json(&body)
            .send()?
            .json()?)
    }

    pub fn fetch_problem(&self, slug: &str) -> Result<Problem> {
        let query = FETCH_PROBLEM_QUERY;
        let variables = serde_json::json!({
            "titleSlug": slug
        });
        let ret = self.query_graphql(query, variables)?;
        Ok(Problem::from_graphql_value(&ret, slug)?)
    }

    /// Runs a solution against a problem's visible example testcases via
    /// LeetCode's `interpret_solution/` endpoint (the "Run" button, not a
    /// real submission)
    pub fn run_testcases(&self, problem: &Problem, solution: &ParsedSolution) -> Result<RunResult> {
        let slug = &problem.title_slug;
        let body = serde_json::json!({
            "lang": solution.lang.as_str(),
            "question_id": &solution.question_id,
            "typed_code": solution.typed_code,
            "data_input": problem.example_testcase_list.join("\n")
        });

        let interpret_id = self.start_judge(slug, "interpret_solution", &body)?;
        self.poll_until_judgement::<RunResult>(&interpret_id, slug)
    }

    pub fn submit_solution(
        &self,
        problem: &Problem,
        solution: &ParsedSolution,
    ) -> Result<SubmissionResult> {
        let slug = &problem.title_slug;
        let body = serde_json::json!({
            "lang": solution.lang.as_str(),
            "question_id": &solution.question_id,
            "typed_code": solution.typed_code,
        });

        let submission_id = self.start_judge(slug, "submit", &body)?;
        let status = self.poll_until_judgement::<SubmissionStatus>(&submission_id, slug)?;
        Ok(status.into())
    }

    pub fn fetch_problem_list(&self) -> Result<Vec<ProblemSummary>> {
        const PROBLEM_PAGE_SIZE: i32 = 100;

        let mut all_problems = Vec::new();
        let mut skip = 0;

        loop {
            let (page, total) = self.fetch_problem_page(skip, PROBLEM_PAGE_SIZE)?;
            all_problems.extend(page);

            if all_problems.len() == total as usize {
                break;
            }
            skip += PROBLEM_PAGE_SIZE;
        }

        Ok(all_problems)
    }

    pub fn fetch_problem_page(
        &self,
        skip: i32,
        limit: i32,
    ) -> Result<(Vec<ProblemSummary>, usize)> {
        let variables = serde_json::json!({
            "categorySlug": "",
            "skip": skip,
            "limit": limit,
            "filters": {}
        });

        let raw = self.query_graphql(PROBLEM_LIST_QUERY, variables)?;

        ProblemSummary::list_from_graphql_value(&raw)
    }

    /* ----- private helpers ----- */
    fn with_auth_headers(
        &self,
        builder: reqwest::blocking::RequestBuilder,
        referer: &str,
    ) -> Result<reqwest::blocking::RequestBuilder> {
        let (session, csrf) = self.config.require_session()?;
        Ok(builder
            .header("Referer", referer)
            .header("x-csrftoken", csrf)
            .header(
                "Cookie",
                format!("LEETCODE_SESSION={session}; csrftoken={csrf}"),
            ))
    }

    fn start_judge(&self, slug: &str, path: &str, body: &serde_json::Value) -> Result<String> {
        self.rate_limit();
        let referer = problem_referer(slug);
        let url = format!("{referer}/{path}/");

        let response_text = self
            .with_auth_headers(self.http.post(&url), &referer)?
            .json(body)
            .send()?
            .text()?;

        let response_json: serde_json::Value = serde_json::from_str(&response_text)?;
        let id_field = response_json
            .get("interpret_id")
            .or_else(|| response_json.get("submission_id"))
            .and_then(value_to_id_string)
            .expect("response was not in the expected shape: possibly a bot check");

        Ok(id_field.to_string())
    }

    /// polls `/submissions/detail/<id>/check/` until the run finishes,
    /// then deserializes the final response into `RunResult`.
    fn poll_until_judgement<T: serde::de::DeserializeOwned>(
        &self,
        id: &str,
        slug: &str,
    ) -> Result<T> {
        let url = format!("https://leetcode.com/submissions/detail/{id}/check/");

        for _ in 0..20 {
            let response: serde_json::Value = self
                .with_auth_headers(self.http.get(&url), &problem_referer(slug))?
                .header("Content-Type", "application/json")
                .send()?
                .json()?;

            let state = response.get("state").and_then(|v| v.as_str()).unwrap_or("");

            // break when finished judging
            if state == "SUCCESS" {
                return Ok(serde_json::from_value(response)?);
            }

            // limit polling rate to avoid blockout
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        // shouldn't get here: likely a hanging server issue
        Err(LeetCodeError::TestingTooLong)
    }

    fn rate_limit(&self) {
        const MIN_INTERVAL: Duration = Duration::from_millis(500);
        let mut last = self.last_request.borrow_mut();
        let elapsed = last.elapsed();
        if elapsed < MIN_INTERVAL {
            std::thread::sleep(MIN_INTERVAL - elapsed);
        }
        *last = Instant::now();
    }
}

// referer is always the same: the associated problem
fn problem_referer(slug: &str) -> String {
    format!("https://leetcode.com/problems/{slug}")
}

fn value_to_id_string(value: &serde_json::Value) -> Option<String> {
    if let Some(s) = value.as_str() {
        return Some(s.to_string());
    }
    if let Some(n) = value.as_i64() {
        return Some(n.to_string());
    }
    None
}

/*
 * ========== UNIT TESTS ==========
 */
#[cfg(test)]
mod client_tests {
    use super::*;
    use crate::models::LangSlug;

    fn live_client() -> LeetCodeClient {
        LeetCodeClient::new().expect("client should construct")
    }

    fn two_sum_question(client: &LeetCodeClient) -> Problem {
        client
            .fetch_problem("two-sum")
            .expect("should fetch two-sum")
    }

    #[test]
    #[ignore = "hits the real LeetCode API — run manually with `cargo test -- --ignored --nocapture`"]
    fn run_testcases_reports_accepted_for_correct_solution() {
        let client = live_client();
        let question = two_sum_question(&client);

        let solution = ParsedSolution {
            lang: LangSlug::Rust,
            question_id: question.question_id.clone(),
            typed_code: r#"
 impl Solution {
     pub fn two_sum(nums: Vec<i32>, target: i32) -> Vec<i32> {
         use std::collections::HashMap;
         let mut seen = HashMap::new();
         for (i, &n) in nums.iter().enumerate() {
             if let Some(&j) = seen.get(&(target - n)) {
                 return vec![j as i32, i as i32];
             }
             seen.insert(n, i);
         }
         vec![]
     }
 }
 "#
            .to_string(),
        };

        let result = client
            .run_testcases(&question, &solution)
            .expect("run_testcases should succeed");

        assert_eq!(result.status_code, 10);
        assert_eq!(result.status_msg, "Accepted");
        assert_eq!(result.correct_answer, Some(true));
        assert_eq!(result.total_correct, result.total_testcases);
    }

    #[test]
    #[ignore = "hits the real LeetCode API — run manually with `cargo test -- --ignored --nocapture`"]
    fn run_testcases_reports_wrong_answer_for_bad_solution() {
        let client = live_client();
        let question = two_sum_question(&client);

        let solution = ParsedSolution {
            lang: LangSlug::Rust,
            question_id: question.question_id.clone(),
            typed_code: r#"
 impl Solution {
     pub fn two_sum(nums: Vec<i32>, target: i32) -> Vec<i32> {
         vec![0, 0]
     }
 }
 "#
            .to_string(),
        };

        let result = client
             .run_testcases(&question, &solution)
             .expect("run_testcases should succeed (as an API call — the *solution* is wrong, not the request)");

        assert_eq!(result.correct_answer, Some(false));
    }

    #[test]
    #[ignore = "hits the real LeetCode API — run manually with `cargo test -- --ignored --nocapture`"]
    fn run_testcases_reports_compile_error_for_invalid_syntax() {
        let client = live_client();
        let question = two_sum_question(&client);

        let solution = ParsedSolution {
            lang: LangSlug::Rust,
            question_id: question.question_id.clone(),
            typed_code: r#"
 impl Solution {
     pub fn two_sum(nums: Vec<i32>, target: i32) -> Vec<i32> {
         shouldn'tcompile
     }
 }
 "#
            .to_string(),
        };

        let result = client
             .run_testcases(&question, &solution)
             .expect("run_testcases should succeed (as an API call — the *code* fails to compile, not the request)");

        assert_eq!(result.status_msg, "Compile Error");
        assert!(result.compile_error.is_some());
        assert_eq!(result.correct_answer, None);
    }
}

#[cfg(test)]
mod submit_tests {
    use super::*;
    use crate::models::{LangSlug, SubmissionResult};

    fn live_client() -> LeetCodeClient {
        LeetCodeClient::new().expect("client should construct")
    }

    fn two_sum_question(client: &LeetCodeClient) -> Problem {
        client
            .fetch_problem("two-sum")
            .expect("should fetch two-sum")
    }

    #[test]
    #[ignore = "hits the real LeetCode API and records a real submission — run manually with `cargo test -- --ignored --nocapture`"]
    fn submit_solution_reports_accepted_for_correct_solution() {
        let client = live_client();
        let question = two_sum_question(&client);

        let solution = ParsedSolution {
            lang: LangSlug::Rust,
            question_id: question.question_id.clone(),
            typed_code: r#"
impl Solution {
    pub fn two_sum(nums: Vec<i32>, target: i32) -> Vec<i32> {
        use std::collections::HashMap;
        let mut seen = HashMap::new();
        for (i, &n) in nums.iter().enumerate() {
            if let Some(&j) = seen.get(&(target - n)) {
                return vec![j as i32, i as i32];
            }
            seen.insert(n, i);
        }
        vec![]
    }
}
"#
            .to_string(),
        };

        let result = client
            .submit_solution(&question, &solution)
            .expect("submit_solution should succeed");

        match result {
            SubmissionResult::Accepted {
                total_correct,
                total_testcases,
                runtime_percentile,
                memory_percentile,
                ..
            } => {
                assert_eq!(total_correct, total_testcases);
                assert!(
                    runtime_percentile > 0.0,
                    "expected a real runtime percentile, got {runtime_percentile}"
                );
                assert!(
                    memory_percentile > 0.0,
                    "expected a real memory percentile, got {memory_percentile}"
                );
            }
            other => panic!("expected Accepted, got a different outcome: {other:?}"),
        }
    }

    #[test]
    #[ignore = "hits the real LeetCode API and records a real submission — run manually with `cargo test -- --ignored --nocapture`"]
    fn submit_solution_reports_wrong_answer_for_bad_solution() {
        let client = live_client();
        let question = two_sum_question(&client);

        let solution = ParsedSolution {
            lang: LangSlug::Rust,
            question_id: question.question_id.clone(),
            typed_code: r#"
impl Solution {
    pub fn two_sum(nums: Vec<i32>, target: i32) -> Vec<i32> {
        vec![0, 0]
    }
}
"#
            .to_string(),
        };

        let result = client
            .submit_solution(&question, &solution)
            .expect("submit_solution should succeed (as an API call — the *solution* is wrong, not the request)");

        match result {
            SubmissionResult::WrongAnswer { .. } => {}
            other => panic!("expected WrongAnswer, got a different outcome: {other:?}"),
        }
    }
}

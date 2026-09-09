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
const DAILY_QUESTION_ID_QUERY: &str = "query questionOfToday {
  activeDailyCodingChallengeQuestion {
    question {
      questionFrontendId
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
        variables: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        self.rate_limit();

        let variables = variables.unwrap_or_default();

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
        let ret = self.query_graphql(query, Some(variables))?;
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

        let raw = self.query_graphql(PROBLEM_LIST_QUERY, Some(variables))?;

        ProblemSummary::list_from_graphql_value(&raw)
    }

    pub fn fetch_daily_challenge_id(&self) -> Result<i64> {
        let value = self.query_graphql(DAILY_QUESTION_ID_QUERY, None)?;
        let slug = value
            .get("data")
            .and_then(|d| d.get("activeDailyCodingChallengeQuestion"))
            .and_then(|c| c.get("question"))
            .and_then(|q| q.get("questionFrontendId"))
            .and_then(|s| s.as_str())
            .ok_or_else(|| {
                LeetCodeError::MalformedResponse("expected daily challenge slug".to_string())
            })?;

        Ok(slug.parse()?)
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

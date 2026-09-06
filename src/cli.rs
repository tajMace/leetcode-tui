// clap arg definitions: `pull <slug>`, `test <slug>`, `submit <slug>`

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::models::LangSlug;

#[derive(Debug, Subcommand)]
pub enum Command {
    Pull {
        slug: String,
        #[arg(long, value_enum)]
        lang: LangSlug,
    },
    Test {
        slug: String,
        #[arg(long, value_enum)]
        lang: LangSlug,
    },
    Submit {
        slug: String,
        #[arg(long, value_enum)]
        lang: LangSlug,
    },
    Init {
        path: PathBuf,
    },
    Login,
    List,
    Refresh,
    Remove {
        slug: String,
        #[arg(long)]
        lang: Option<LangSlug>,
    },
}

#[derive(Debug, Parser)]
#[command(
    name = "leetcode-cli",
    about = "Pull, test, and submit LeetCode problems from the terminal"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

// entrypoint — parse CLI args, load config, dispatch to commands

mod cache;
mod cli;
mod client;
mod commands;
mod config;
mod error;
mod manifest;
mod models;
mod tui;

use clap::Parser;
use cli::Cli;

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        cli::Command::Pull { slug, lang } => commands::pull(slug, lang),
        cli::Command::Test { slug, lang } => commands::test(slug, lang),
        cli::Command::Submit { slug, lang } => commands::submit(slug, lang),
        cli::Command::Init { path } => commands::init(path),
        cli::Command::Login => commands::login(),
        cli::Command::List => commands::list(),
        cli::Command::Refresh => cache::download_and_save_problem_list(),
        cli::Command::Remove { slug, lang } => commands::remove(slug, lang),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

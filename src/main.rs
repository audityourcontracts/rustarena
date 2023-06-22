mod html_parsing;
mod github_api;
mod contract;
mod cli;
mod parser_c4;
mod parser_sherlock;

use cli::Cli;
use env_logger;

fn main() {
    env_logger::init();
    let cli = Cli::new();
    cli.run();
}
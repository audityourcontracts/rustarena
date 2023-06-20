mod html_parsing;
mod github_api;
mod contract;
mod cli;
mod c4parser;

use cli::Cli;
use env_logger;

fn main() {
    env_logger::init();
    let cli = Cli::new();
    cli.run();
}
mod github_api;
mod contract;
mod cli;
mod parsers {
    pub mod parse;
    pub mod sherlock;
    pub mod code4rena;
    pub mod immunefi;
    pub mod hats;
}
mod builders {
    pub mod build;
    pub mod forge;
    pub mod hardhat;
}

use cli::Cli;
use env_logger;

fn main() {
    env_logger::init();
    let cli = Cli::new();
    cli.run();
}
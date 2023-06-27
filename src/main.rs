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
    pub mod truffle;
}

use cli::Cli;
use env_logger;

#[tokio::main]
async fn main() {
    // TODO: Come back and filter this ;)
    /*Builder::new()
    .filter(Some("headless_chrome"), LevelFilter::Debug)
    .init();
    */
    env_logger::init();
    let cli = Cli::new();
    cli.run().await;
}
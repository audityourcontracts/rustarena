use crate::html_parsing;
use crate::github_api;
use crate::contract::{process_repository, process_out_directory};
use clap::Parser;
use log;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    website: String,
}

pub struct Cli {
}

impl Cli {
    pub fn new() -> Self {
        Self {
            // Initialize fields as needed
        }
    }

    pub fn run(&self) {
        let args = Args::parse();
        let website_url = &args.website;

        // Add some information logging 
        log::info!("Parsing website {}", website_url);
        match html_parsing::parse_dom(website_url) {
            Ok(repos) => {
                for repo in repos {
                    match github_api::clone_repository(&repo.url, &repo.name) {
                        Ok(_) => {
                            log::info!("Repo cloned from {} to {}", &repo.url, &repo.name);
                            process_repository(&repo.name);
                            let contract_data = process_out_directory(&repo.name);

                            // Print repository name
                            println!("Repository: {}", &repo.name);

                            for (contract_name, bytecode) in contract_data {
                                // Print contract name and bytecode
                                println!("Contract: {}", contract_name);
                                println!("Bytecode: {:?}", bytecode);
                            }
                        }
                        Err(err) => {
                            eprintln!("Error cloning repo: {}", err);
                        }
                    }
                }
            }
            Err(err) => {
                eprintln!("Error parsing website: {}", err);
            }
        }
    }
}

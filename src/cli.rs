use crate::parsers::parse::{WebsiteParser};
use crate::parsers::code4rena::Code4renaParser;
use crate::parsers::sherlock::SherlockParser;
use crate::parsers::immunefi::ImmunefiParser;
use crate::parsers::hats::HatsParser;
use crate::github_api;
use crate::contract::{process_repository, ContractKind};
use clap::Parser;
use log;
use crate::parsers::parse::Repo;
use url::Url;
use tokio::task::{spawn, spawn_blocking};
use std::sync::{Arc};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    truncate: bool,

    #[arg(short, long)]
    github: Option<String>,
}

pub struct Cli {
}

impl Cli {
    pub fn new() -> Self {
        Self {
        }
    }

    pub async fn run(&self) {
        let args = Args::parse();

        let parsers: Vec<Arc<dyn WebsiteParser + Sync + Send>> = vec![
            Arc::new(Code4renaParser::new()),
            Arc::new(SherlockParser::new()),
            Arc::new(ImmunefiParser::new()),
            Arc::new(HatsParser::new()),
        ];

        if let Some(github_link) = &args.github {
            let repo_name = format!("repos/{}", get_last_path_part(&github_link.as_str()).unwrap());
            // Process a single GitHub repository
            let repo = Repo {
                name: repo_name,
                url: github_link.clone(),
                commit: None,
            };
            // Not sure this is right, how do I know the task returned.
            spawn_blocking(move || {
                if let Err(err) = process_results(&repo, args.truncate) {
                    log::error!("Error processing repository: {}", err);
                }
            });
        } else {
            let tasks = parsers.into_iter().map(|parser| {
                spawn_blocking(move || {
                    log::info!("Parsing website {}", parser.url());
                    // Parse the dom, clone the repo, process the repo, print the results
                    match parser.parse_dom() {
                        Ok(repos) => {
                            for repo in repos {
                                spawn(async move {
                                    if let Err(err) = process_results(&repo, args.truncate) {
                                        log::error!("Error processing repository: {}", err);
                                    }
                                });
                            }
                        }
                        Err(err) => {
                            log::error!("Error parsing website: {}", err);
                        }
                    }
                })
            });
            futures::future::join_all(tasks).await; 
        }
        
    }
}

fn process_results(repo: &Repo, truncate: bool) -> Result<(), Box<dyn std::error::Error>> {
    match github_api::clone_repository(&repo) {
        Ok(_) => {
            match process_repository(&repo.name) {
                Ok((repo_name, contract_data)) => {
                    let mut sorted_contracts = contract_data;
                    sorted_contracts.sort_by_key(|contract| match contract.contract_kind {
                        ContractKind::Interface => 0,
                        ContractKind::Contract => 1,
                    });

                    // Enumerate the Vec<Contract> received by calling process_out_directory
                    for contract in sorted_contracts {
                        println!("Repository: {}", repo_name);
                        println!("Contract Name: {}", contract.contract_name);
                        match &contract.imports {
                            Some(imports) => println!("Number of imports: {}", imports.len()),
                            None => println!("Number of imports: 0"),
                        }
                        match contract.contract_kind {
                            ContractKind::Interface => {
                                println!("Contract Type: Interface");
                                print_bytecode(contract.bytecode, truncate);
                            }
                            ContractKind::Contract => {
                                println!("Contract Type: Contract");
                                print_bytecode(contract.bytecode, truncate);
                            }
                        }
                    }
                }
                Err(err) => {
                    log::error!("Error processing repository: {}", err);
                }
            }
        }
        Err(err) => {
            log::error!("Error cloning repo: {}", err);
        }
    }
    Ok(())
}    

fn print_bytecode(bytecode: String, truncate: bool) {
    if truncate {
        let truncated_bytecode = truncate_bytecode(&bytecode);
        println!("Bytecode: {}", truncated_bytecode);
    } else {
        println!("Bytecode: {}", bytecode);
    }
}

fn truncate_bytecode(bytecode: &str) -> String {
    const MAX_BYTECODE_LENGTH: usize = 100;
    if bytecode.len() > MAX_BYTECODE_LENGTH {
        format!("{}...", &bytecode[..MAX_BYTECODE_LENGTH])
    } else {
        bytecode.to_owned()
    }
}

fn get_last_path_part(url: &str) -> Option<String> {
    if let Ok(parsed_url) = Url::parse(url) {
        parsed_url.path_segments()?.last().map(String::from)
    } else {
        None
    }
}
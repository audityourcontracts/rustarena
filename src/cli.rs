use crate::html_parsing::{WebsiteParser};
use crate::parser_c4::{Code4renaParser};
use crate::github_api;
use crate::contract::{process_repository, process_out_directory, ContractKind};
use clap::Parser;
use log;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    truncate: bool,
}

pub struct Cli {
}

impl Cli {
    pub fn new() -> Self {
        Self {
        }
    }

    pub fn run(&self) {
        let args = Args::parse();

        let c4parser = Code4renaParser::new();

        // Add some information logging 
        log::info!("Parsing website {}", c4parser.url);
        // Parse the dom, clone the repo, process the repo, print the results
        match c4parser.parse_dom() {
            Ok(repos) => {
                for repo in repos {
                    match github_api::clone_repository(&repo.url, &repo.name) {
                        Ok(_) => {
                            log::info!("Repo cloned from {} to {}", &repo.url, &repo.name);
                            process_repository(&repo.name);
                            let (repo_name, contract_data) = process_out_directory(&repo.name);

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
                                        Self::print_bytecode(contract.bytecode, args.truncate);
                                    }
                                    ContractKind::Contract => {
                                        println!("Contract Type: Contract");
                                        Self::print_bytecode(contract.bytecode, args.truncate);
                                    }
                                }
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

    fn print_bytecode(bytecode: String, truncate: bool) {
        if truncate {
            let truncated_bytecode = Self::truncate_bytecode(&bytecode);
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

}

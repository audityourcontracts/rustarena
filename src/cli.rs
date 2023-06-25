use crate::parsers::parse::{WebsiteParser};
use crate::parsers::code4rena::Code4renaParser;
use crate::parsers::sherlock::SherlockParser;
use crate::parsers::immunefi::ImmunefiParser;
use crate::github_api;
use crate::contract::{process_repository, ContractKind};
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

        let parsers: Vec<Box<dyn WebsiteParser>> = vec![
            Box::new(Code4renaParser::new()),
            Box::new(SherlockParser::new()),
            Box::new(ImmunefiParser::new()),
        ];

        for parser in parsers {
            log::info!("Parsing website {}", parser.url());
            // Parse the dom, clone the repo, process the repo, print the results
            match parser.parse_dom() {
                Ok(repos) => {
                    for repo in repos {
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
                                        log::error!("Error processing repository: {}", err);
                                    }
                                }
                            }
                            Err(err) => {
                                log::error!("Error cloning repo: {}", err);
                            }
                        }
                    }
                }
                Err(err) => {
                    log::error!("Error parsing website: {}", err);
                }
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

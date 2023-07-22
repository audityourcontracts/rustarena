use std::path::Path;
use std::fs;
use tokio::task::{spawn_blocking,spawn};
use std::sync::Arc;
use futures::future::try_join_all;
use tokio::sync::Semaphore;
use clap::Parser;
use log;

use crate::parsers::code4rena::Code4renaParser;
use crate::parsers::sherlock::SherlockParser;
use crate::parsers::immunefi::ImmunefiParser;
use crate::parsers::hats::HatsParser;
use crate::github_api;
use crate::contract::{process_repository, Kind};
use crate::parsers::parse::Repo;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    github: Option<String>,

    #[arg(short, long, default_value = "30")]
    max_builders: usize,

    #[arg(short, long, default_value = "false")]
    keep_unsupported: bool,
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

        let mut tasks = Vec::new();

        if let Some(github_link) = &args.github {
            let repo_name = format!("repos/{}", github_api::get_last_path_part(&github_link.as_str()).unwrap());
            // Process a single GitHub repository
            let repo = Repo {
                parser: "github_command_line".to_string(),
                name: repo_name,
                url: github_link.clone(),
                commit: None,
            };
            log::debug!("Initiating Github build for {}", &repo.name);
            spawn_blocking(move || {
                if let Err(err) = process_results(&repo, args.keep_unsupported) {
                    log::error!("Error processing repository: {}", err);
                }
            });
        } else {
            tasks.push(spawn({
                let immunefi = Arc::new(ImmunefiParser::new());
                async move {
                    immunefi.parse_dom().await
                }
            }));

            tasks.push(spawn({
                let code4rena = Arc::new(Code4renaParser::new());
                async move {
                    code4rena.parse_dom().await
                }
            }));

            tasks.push(spawn({
                let sherlock = Arc::new(SherlockParser::new());
                async move {
                    sherlock.parse_dom().await
                }
            }));

            tasks.push(spawn({
                let hats = Arc::new(HatsParser::new());
                async move {
                    hats.parse_dom().await
                }
            }));
            
            //Set the maximum number of concurrent builders.
            let semaphore = Arc::new(Semaphore::new(args.max_builders));
            
            let builder_tasks = try_join_all(tasks)
                .await
                .unwrap()
                .into_iter()
                .flat_map(|result| result.unwrap())
                .map(|repo| {
                    let semaphore = Arc::clone(&semaphore);
                    // Spawn a task for each repository
                    spawn(async move {
                        let permit = semaphore.acquire().await.expect("Failed to acquire semaphore permit");
                        if let Err(err) = process_results(&repo, args.keep_unsupported) {
                            log::error!("Error processing repository: {}", err);
                        }
                        drop(permit);
                    })
                })
                .collect::<Vec<_>>();
            
            try_join_all(builder_tasks).await.unwrap();
        }
        
    }
}

fn process_results(repo: &Repo, keep_unsupported: bool) -> Result<(), Box<dyn std::error::Error>> {
    match github_api::clone_repository(&repo) {
        Ok(_) => {
            match process_repository(&repo, keep_unsupported) {
                Ok((_repo_name, contract_data)) => {
                    if contract_data.len() > 0 {
                        let mut sorted_contracts = contract_data;
                        sorted_contracts.sort_by_key(|contract| match contract.kind {
                            Kind::Interface => 0,
                            Kind::Contract => 1,
                        });
    
                        // Create a results directory if it doesn't exist. 
                        let results_dir = Path::new("results");
                        if !results_dir.exists() {
                            fs::create_dir(results_dir)?;
                        }
    
                        // Serialize and write the sorted contracts to a JSON file
                        let repo_path = Path::new(&repo.name).strip_prefix("repos")?;
                        let json_data = serde_json::to_string_pretty(&sorted_contracts)?;
                        let json_filename = format!("results/{}_{}_contracts.json", &repo.parser, &repo_path.to_string_lossy());
                        log::debug!("Writing {}", &json_filename);
                        fs::write(json_filename, json_data)?;
                    } else {
                        log::error!("No contract output for {}", &repo.name);
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
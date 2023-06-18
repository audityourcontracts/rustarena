use std::process::{Command, exit};
use log;
use std::path::{Path};
use std::fs;
use std::env;

use serde_derive::Deserialize;
use serde_derive::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutDirectory {
    pub abi: Vec<Abi>,
    pub bytecode: Bytecode,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Abi {
    pub inputs: Vec<Input>,
    pub state_mutability: Option<String>,
    #[serde(rename = "type")]
    pub type_field: String,
    pub name: Option<String>,
    pub anonymous: Option<bool>,
    #[serde(default)]
    pub outputs: Vec<Output>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Input {
    pub internal_type: String,
    pub name: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub indexed: Option<bool>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    pub internal_type: String,
    pub name: String,
    #[serde(rename = "type")]
    pub type_field: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bytecode {
    pub object: String,
    pub source_map: String,
    pub link_references: LinkReferences,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkReferences {
}

pub fn process_repository(repo_directory: &str) {
    log::info!("Executing forge install in {}", repo_directory);
    let install_result = Command::new("forge")
        .arg("install")
        .current_dir(repo_directory)
        .output();

    if let Err(err) = install_result {
        eprintln!("Error executing 'forge install': {}", err);
        exit(1);
    }

    // Execute `forge build` in the repository directory
    log::info!("Executing forge build in {}", repo_directory);
    let build_result = Command::new("forge")
        .arg("build")
        .current_dir(repo_directory)
        .output();

    if let Err(err) = build_result {
        eprintln!("Error executing 'forge build': {}", err);
        exit(1);
    }

    if let Ok(current_dir) = env::current_dir() {
        log::info!("Current directory: {}", current_dir.to_string_lossy());
    } else {
        log::error!("Failed to get current directory");
    }

    let cache_dir = Path::new(&repo_directory).join("cache");
    if !cache_dir.exists() || !cache_dir.is_dir() {
        eprintln!("Error: 'cache' directory not found in {}", repo_directory);
        exit(1);
    }
    
    // Check if "out" directory exists
    let out_dir = Path::new(&repo_directory).join("out");
    log::info!("Checking for the out directory {}", out_dir.to_string_lossy());
    if !out_dir.exists() {
        eprintln!("Error: 'out' directory {} not found in {}", out_dir.to_string_lossy(), repo_directory);
        exit(1);
    }
}

// To sort by the type of contract. Interfaces have the bytecode 0x
pub struct Contract {
    pub contract_name: String,
    pub contract_kind: ContractKind,
    pub bytecode: String,
}
pub enum ContractKind {
    Interface,
    Contract,
}

pub fn process_out_directory(repo_directory: &str) -> (String, Vec<Contract>) {
    let out_dir = Path::new(&repo_directory).join("out");
    log::info!("Looking for build contracts in {}", &out_dir.to_string_lossy());
    let mut results = Vec::new();

    if let Ok(entries) = fs::read_dir(&out_dir) {
        let mut entry_count = 0;
        
        for entry in entries.flatten() {
            if let Ok(entry_path) = entry.path().canonicalize() {
                if let Some(contract_name) = entry_path.file_name().and_then(|name| name.to_str().map(|s| s.trim_end_matches(".sol"))) {
                    let json_file = entry_path.join(format!("{}.json", contract_name));
                    log::debug!("Processing JSON file: {}", json_file.to_string_lossy());
        
                    if let Ok(json_content) = fs::read_to_string(&json_file) {
                        let out_directory: OutDirectory = serde_json::from_str(&json_content).unwrap_or_default();
                        let bytecode_object = out_directory.bytecode.object;

                        let contract_kind = if bytecode_object.starts_with("0x") {
                            ContractKind::Interface
                        } else {
                            ContractKind::Contract
                        };
        
                        results.push(Contract {
                            contract_name: contract_name.to_owned(),
                            contract_kind,
                            bytecode: bytecode_object.to_owned(),
                        });
                    } else {
                        //eprintln!("Error parsing JSON file");
                    }
                } else {
                    eprintln!("Error reading JSON file");
                } 
            } 
            entry_count += 1;
        }
        log::info!("Number of entries found: {}", entry_count);
    } else {
        eprintln!("Error getting directory entries");
    }
    (repo_directory.to_owned(), results)
}


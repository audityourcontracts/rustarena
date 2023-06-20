use std::process::{Command, exit};
use log;
use std::path::{Path};
use std::fs;
use std::env;
use std::collections::HashMap;

use serde_derive::Deserialize;
use serde_derive::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub abi: Vec<Abi>,
    pub bytecode: Bytecode,
    pub ast: Ast,
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

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ast {
    pub absolute_path: String,
    pub id: i64,
    pub node_type: String,
    pub src: String,
    pub nodes: Vec<Node>,
    pub license: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: i64,
    pub node_type: String,
    pub src: String,
    pub nodes: Vec<Node>,
    pub literals: Option<Vec<String>>,
    pub absolute_path: Option<String>,
    pub file: Option<String>,
    pub name_location: Option<String>,
    pub scope: Option<i64>,
    pub source_unit: Option<i64>,
    #[serde(default)]
    pub symbol_aliases: Vec<SymbolAliases>,
    pub unit_alias: Option<String>,
    #[serde(rename = "abstract")]
    pub abstract_field: Option<bool>,
    #[serde(default)]
    pub canonical_name: Option<String>,
    #[serde(default)]
    pub contract_dependencies: Vec<i64>,
    pub contract_kind: Option<String>,
    pub fully_implemented: Option<bool>,
    pub linearized_base_contracts: Option<Vec<i64>>,
    pub name: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolAliases {
    pub foreign: Foreign,
    pub name_location: String,
    pub local: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Foreign {
    pub id: i64,
    pub name: String,
    pub node_type: String,
    pub referenced_declaration: i64,
    pub src: String,
}

// Contract struct. ContractKind for Contract vs Interfaces. Interfaces have the bytecode 0x
#[derive(Clone, Debug)]
pub struct Contract {
    pub contract_name: String,
    pub contract_kind: ContractKind,
    pub bytecode: String,
    pub imports: Option<Vec<Contract>>,
}
#[derive(Clone, Debug)]
pub enum ContractKind {
    Interface,
    Contract,
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

pub fn process_out_directory(repo_directory: &str) -> (String, Vec<Contract>) {
    let out_dir = Path::new(&repo_directory).join("out");
    log::info!("Looking for build contracts in {}", &out_dir.to_string_lossy());

    // Contract map stores a mapping from contract name to Contract.
    let mut contract_map: HashMap<String, Contract> = HashMap::new();

    // The results we will return.
    //let mut results = Vec::new();

    if let Ok(entries) = fs::read_dir(&out_dir) {
        let mut entry_count = 0;
        
        for entry in entries.flatten() {
            if let Ok(entry_path) = entry.path().canonicalize() {
                if let Some(contract_name) = entry_path.file_name().and_then(|name| name.to_str().map(|s| s.trim_end_matches(".sol"))) {
                    let json_file = entry_path.join(format!("{}.json", contract_name));
                    log::debug!("Processing JSON file: {}", json_file.to_string_lossy());
        
                    if let Ok(json_content) = fs::read_to_string(&json_file) {
                        let metadata: Metadata = serde_json::from_str(&json_content).unwrap_or_default();
                        let bytecode_object = metadata.bytecode.object;

                        let contract_kind = if bytecode_object.eq("0x") {
                            ContractKind::Interface
                        } else {
                            ContractKind::Contract
                        };

                        let contract = Contract {
                            contract_name: contract_name.to_owned(),
                            contract_kind,
                            bytecode: bytecode_object.to_owned(),
                            imports: None, // first pass no imports.
                        };
                        contract_map.insert(contract_name.to_owned(), contract.to_owned());
                        //results.push(contract.to_owned());

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

    // Add contract imports. This means reading all the directories again
    // And for each import for a given contract
    // Retrieve the contract from the hash map and add all of it's imports.
    // This should result in a hashmap of String (contract name) to Contract.
    // Then I can turn this HashMap into a Vec<Contract> and return it.

    if let Ok(entries) = fs::read_dir(&out_dir) {
        let mut entry_count = 0;

        let contract_map_clone = contract_map.clone();
        
        for entry in entries.flatten() {
            if let Ok(entry_path) = entry.path().canonicalize() {
                if let Some(contract_name) = entry_path.file_name().and_then(|name| name.to_str().map(|s| s.trim_end_matches(".sol"))) {
                    let json_file = entry_path.join(format!("{}.json", contract_name));
                    log::debug!("Processing JSON file: {}", json_file.to_string_lossy());
        
                    if let Ok(json_content) = fs::read_to_string(&json_file) {
                        let metadata: Metadata = serde_json::from_str(&json_content).unwrap_or_default();

                        // Read the imports for the current contract_name
                        // Retrieve the contract name from the HashMap and update its imports.
                        if let Some(contract) = contract_map.get_mut(contract_name) {
                            for node in metadata.ast.nodes {
                                if node.node_type == "ImportDirective" {
                                    for symbol_alias in node.symbol_aliases {
                                        let foreign_name = symbol_alias.foreign.name.clone();
                    
                                        if let Some(imported_contract) = contract_map_clone.get(&foreign_name) {
                                            if contract.imports.is_none() {
                                                contract.imports = Some(Vec::new());
                                            }
                                            if let Some(imports) = contract.imports.as_mut() {
                                                imports.push(imported_contract.clone());
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            eprintln!("Error retrieving contract from HashMap");
                        }
                    } else {
                        eprintln!("Error parsing JSON file");
                    }
                } else {
                    eprintln!("Error reading JSON file");
                } 
            } 
            entry_count += 1;
        }
        log::info!("Number of entries found: {}", entry_count);
    }

    // contract_map should have all contracts witha all imports
    let contracts: Vec<Contract> = contract_map.values().cloned().collect();
    (repo_directory.to_owned(), contracts)
}
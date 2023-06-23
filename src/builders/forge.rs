use std::process::{Command, exit};
use log;
use std::path::{Path};
use std::fs;
use std::env;
use std::collections::HashMap;
use walkdir::WalkDir;

use serde_derive::Deserialize;
use serde_derive::Serialize;

use crate::builders::build::Build;
use crate::contract::{Contract, ContractKind};

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
    pub inputs: Option<Vec<Input>>,
    pub state_mutability: Option<String>,
    #[serde(rename = "type")]
    pub type_field: String,
    pub name: Option<String>,
    pub anonymous: Option<bool>,
    #[serde(default)]
    pub outputs: Option<Vec<Output>>
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
    pub source_map: Option<String>,
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
    pub license: Option<String>,
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
    pub referenced_declaration: Option<i64>,
    pub src: String,
} 
pub struct ForgeBuilder;

impl Build for ForgeBuilder {
    fn build(&self, directory: &str) -> Result<(String, Vec<Contract>), Box<dyn std::error::Error>> {
        log::info!("Executing forge install in {}", directory);
        let install_result = Command::new("forge")
            .arg("install")
            .current_dir(directory)
            .output();
    
        if let Err(err) = install_result {
            eprintln!("Error executing 'forge install': {}", err);
            exit(1);
        }
    
        // Execute `forge build` in the repository directory
        log::info!("Executing forge build in {}", directory);
        let build_result = Command::new("forge")
            .arg("build")
            .current_dir(directory)
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
    
        let cache_dir = Path::new(&directory).join("cache");
        if !cache_dir.exists() || !cache_dir.is_dir() {
            eprintln!("Error: 'cache' directory not found in {}", directory);
            //exit(1);
        }
        
        // Check if "out" directory exists
        let out_dir = Path::new(&directory).join("out");
        log::info!("Checking for the out directory {}", out_dir.to_string_lossy());
        if !out_dir.exists() {
            eprintln!("Error: 'out' directory {} not found in {}", out_dir.to_string_lossy(), directory);
            //exit(1);
        }

        let result = process_out_directory(directory);
        Ok(result)
        
    }

 
}

pub fn process_out_directory(repo_directory: &str) -> (String, Vec<Contract>) {
    let out_dir = Path::new(&repo_directory).join("out");
    log::info!("Looking for built contracts in {}", &out_dir.to_string_lossy());

    // Contract map stores a mapping from contract name to Contract.
    let mut contract_map: HashMap<String, Contract> = HashMap::new();

    let walker = WalkDir::new(&out_dir).into_iter();

    // First pass will find all json files, parse them and add them to a contract_map
    // Imports are None at this stage as they are populated in the second pass. 

    for entry in walker.flatten() {
        let entry_path = entry.path();

        if entry_path.is_file() {
            if let Some(extension) = entry_path.extension() {
                if extension == "json" {
                    if let Some(file_stem) = entry_path.file_stem() {
                        if let Some(contract_name) = file_stem.to_str() {
                            let json_content = match fs::read_to_string(&entry_path) {
                                Ok(content) => content,
                                Err(err) => {
                                    eprintln!("Error reading JSON file '{}': {}", entry_path.display(), err);
                                    continue;
                                }
                            };

                            let metadata: Metadata = match serde_json::from_str(&json_content) {
                                Ok(metadata) => metadata,
                                Err(err) => {
                                    eprintln!("Error parsing JSON file '{}': {}", entry_path.display(), err);
                                    continue;
                                }
                            };

                            let bytecode_object = metadata.bytecode.object;

                            let contract_kind = if bytecode_object == "0x" {
                                ContractKind::Interface
                            } else {
                                ContractKind::Contract
                            };

                            let contract = Contract {
                                contract_name: contract_name.to_owned(),
                                contract_kind,
                                bytecode: bytecode_object.to_owned(),
                                imports: None,
                            };
                            contract_map.insert(contract_name.to_owned(), contract);
                        }
                    }
                }
            }
        }
    }

    // In the second pass we read the json file, parse the imports.
    // Then look for the contract in the hashmap and if it's there 
    // We append the imports to the contract's imports field.

    let contract_map_clone = contract_map.clone();

    let walker = WalkDir::new(&out_dir).into_iter();

    for entry in walker.flatten() {
        let entry_path = entry.path();

        if entry_path.is_file() && entry_path.extension() == Some("json".as_ref()) {
            if let Some(file_stem) = entry_path.file_stem() {
                if let Some(contract_name) = file_stem.to_str() {
                    let json_content = match fs::read_to_string(&entry_path) {
                        Ok(content) => content,
                        Err(err) => {
                            eprintln!("Error reading JSON file '{}': {}", entry_path.display(), err);
                            continue;
                        }
                    };

                    let metadata: Metadata = match serde_json::from_str(&json_content) {
                        Ok(metadata) => metadata,
                        Err(parseerr) => {
                            eprintln!("Error parsing JSON file '{}': {}", entry_path.display(), parseerr);
                            continue;
                        }
                    };

                    if let Some(contract) = contract_map.get_mut(contract_name) {
                        for node in metadata.ast.nodes {
                            if node.node_type == "ImportDirective" {
                                for symbol_alias in node.symbol_aliases {
                                    let foreign_name = symbol_alias.foreign.name.clone();

                                    if let Some(imported_contract) = contract_map_clone.get(&foreign_name) {
                                        contract.imports.get_or_insert_with(Vec::new).push(imported_contract.clone());
                                    }
                                }
                            }
                        }
                    } else {
                        eprintln!("Error retrieving contract from HashMap");
                    }
                }
            }
        }
    }
    // contract_map should have all contracts with all imports
    let contracts: Vec<Contract> = contract_map.values().cloned().collect();
    (repo_directory.to_owned(), contracts)
}
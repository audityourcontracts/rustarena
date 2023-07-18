use std::process::{Command, exit};
use log;
use std::path::Path;
use std::fs;
use std::env;
use std::collections::HashMap;
use walkdir::WalkDir;
use serde_derive::Deserialize;
use serde_derive::Serialize;

use crate::builders::build::Build;
use crate::contract::{Contract, Kind};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata{
    pub contract_name: String,
    //pub abi: Vec<Value>,
    pub metadata: String,
    pub bytecode: String,
    pub deployed_bytecode: String,
    pub immutable_references: Option<ImmutableReferences>,
    //pub generated_sources: Vec<Value>,
    //pub deployed_generated_sources: Vec<Value>,
    pub source_map: String,
    pub deployed_source_map: String,
    pub source: String,
    pub source_path: String,
    pub ast: Ast,
    //#[serde(rename = "legacyAST")]
    //pub legacy_ast: LegacyAst,
    //pub compiler: Compiler,
    //pub networks: Networks,
    pub schema_version: String,
    pub updated_at: String,
    //pub devdoc: Devdoc,
    //pub userdoc: Userdoc,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImmutableReferences {
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ast {
    pub absolute_path: Option<String>,
    //pub exported_symbols: ExportedSymbols,
    pub id: i64,
    pub license: Option<String>,
    pub node_type: String,
    pub nodes: Vec<Node>,
    pub src: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub absolute_path: Option<String>,
    pub file: Option<String>,
    pub id: i64,
    pub node_type: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Documentation {
    pub id: i64,
    pub node_type: String,
    pub src: String,
    pub text: String,
}
pub struct TruffleBuilder;

impl Build for TruffleBuilder {
    fn build(&self, directory: &str) -> Result<(String, Vec<Contract>), Box<dyn std::error::Error>> {
        log::info!("Executing npm install in {}", directory);
        let install_result = Command::new("npm")
            .arg("install")
            .current_dir(directory)
            .output();
    
        if let Err(err) = install_result {
            log::error!("Error executing 'npm install': {}", err);
            exit(1);
        }
    
        // Execute `forge build` in the repository directory
        log::info!("Executing truffle compile in {}", directory);
        let build_result = Command::new("truffle")
            .arg("compile")
            .current_dir(directory)
            .output();
    
        if let Err(err) = build_result {
            log::error!("Error executing 'truffle compile': {}", err);
            exit(1);
        }
    
        if let Ok(current_dir) = env::current_dir() {
            log::info!("Current directory: {}", current_dir.to_string_lossy());
        } else {
            log::error!("Failed to get current directory");
        }

        let mut artifact_dir = "";

        let src_dir = Path::new(&directory).join("src");
        if !src_dir.exists() || !src_dir.is_dir() {
            log::error!("Error: 'src' directory not found in {}", directory);
            //exit(1);
        } else {
            artifact_dir = "src";
        }

        // Check if "out" directory exists
        let build_dir = Path::new(&directory).join("build");
        log::info!("Checking for the build directory {}", build_dir.to_string_lossy());
        if !build_dir.exists() {
            log::error!("Error: 'out' directory {} not found in {}", build_dir.to_string_lossy(), directory);
            //exit(1);
        } else {
            artifact_dir = "build";
        }

        let result = process_truffle_directory(directory, artifact_dir);
        Ok(result)
    }
}

pub fn process_truffle_directory(repo_directory: &str, artifact_dir: &str) -> (String, Vec<Contract>) {
    let out_dir = Path::new(&repo_directory).join(artifact_dir);
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
                                    log::error!("Error reading JSON file '{}': {}", entry_path.display(), err);
                                    continue;
                                }
                            };

                            let metadata: Metadata = match serde_json::from_str(&json_content) {
                                Ok(metadata) => metadata,
                                Err(err) => {
                                    log::error!("Error parsing JSON file '{}': {}", entry_path.display(), err);
                                    continue;
                                }
                            };

                            let bytecode_object = metadata.bytecode;

                            let kind = if bytecode_object == "0x" {
                                Kind::Interface
                            } else {
                                Kind::Contract
                            };

                            let contract = Contract {
                                contract_name: contract_name.to_owned(),
                                kind,
                                bytecode: bytecode_object.to_owned(),
                                deployed_bytecode: None,
                                imports: None,
                                sourcemap: None,
                                deployed_sourcemap: None,
                                absolute_path: None,
                                id: None,
                                file_contents: None
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
                            log::error!("Error reading JSON file '{}': {}", entry_path.display(), err);
                            continue;
                        }
                    };

                    let metadata: Metadata = match serde_json::from_str(&json_content) {
                        Ok(metadata) => metadata,
                        Err(parseerr) => {
                            log::error!("Error parsing JSON file '{}': {}", entry_path.display(), parseerr);
                            continue;
                        }
                    };

                    if let Some(contract) = contract_map.get_mut(contract_name) {
                        for node in metadata.ast.nodes {
                            if node.node_type == "ImportDirective" {
                                if let Some(file) = node.file {
                                    let segments: Vec<&str> = file.split('/').collect();
                                    if let Some(last_segment) = segments.last() {
                                        let contract_name = last_segment.trim_end_matches(".sol");
                                        // Add import to contract.
                                        if let Some(imported_contract) = contract_map_clone.get(contract_name) {
                                            contract.imports.get_or_insert_with(Vec::new).push(imported_contract.clone());
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        log::error!("Error retrieving contract from HashMap");
                    }
                }
            }
        }
    }
    // contract_map should have all contracts with all imports
    let contracts: Vec<Contract> = contract_map.values().cloned().collect();
    (repo_directory.to_owned(), contracts)
}
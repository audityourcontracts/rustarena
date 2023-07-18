use std::process::{Command, exit};
use ethers_solc::artifacts::{ImportDirective, NodeType};
use log;
use std::path::{Path};
use std::env;
use std::collections::HashMap;
use walkdir::WalkDir;
use ethers_solc::ConfigurableContractArtifact;

use crate::builders::build::Build;
use crate::contract::{Contract, Kind};

pub struct ForgeBuilder;

impl Build for ForgeBuilder {
    fn build(&self, directory: &str) -> Result<(String, Vec<Contract>), Box<dyn std::error::Error>> {
        log::info!("Executing forge install in {}", directory);
        let install_result = Command::new("forge")
            .arg("install")
            .current_dir(directory)
            .output();
    
        if let Err(err) = install_result {
            log::error!("Error executing 'forge install': {}", err);
            exit(1);
        }
    
        // Execute `forge build` in the repository directory
        log::info!("Executing forge build in {}", directory);
        let build_result = Command::new("forge")
            .arg("build")
            .current_dir(directory)
            .output();
    
        if let Err(err) = build_result {
            log::error!("Error executing 'forge build': {}", err);
            exit(1);
        }
    
        if let Ok(current_dir) = env::current_dir() {
            log::info!("Current directory: {}", current_dir.to_string_lossy());
        } else {
            log::error!("Failed to get current directory");
        }
    
        let cache_dir = Path::new(&directory).join("cache");
        if !cache_dir.exists() || !cache_dir.is_dir() {
            log::error!("Error: 'cache' directory not found in {}", directory);
            //exit(1);
        }
        
        // Check if "out" directory exists
        let out_dir = Path::new(&directory).join("out");
        log::info!("Checking for the out directory {}", out_dir.to_string_lossy());
        if !out_dir.exists() {
            log::error!("Error: 'out' directory {} not found in {}", out_dir.to_string_lossy(), directory);
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
    // Contract imports are None at this stage as they are populated in the second pass. 
    // Not all contracts will be in the map until the first pass is complete.

    for entry in walker.flatten() {
        let entry_path = entry.path();

        if entry_path.is_file() {
            if let Some(extension) = entry_path.extension() {
                if extension == "json" {
                    if let Some(file_stem) = entry_path.file_stem() {
                        if let Some(contract_name) = file_stem.to_str() {
                            let metadata: ConfigurableContractArtifact = match ethers_solc::utils::read_json_file(entry_path) {
                                Ok(metadata) => metadata,
                                Err(err) => {
                                    log::error!("Error reading JSON file '{}': {}", entry_path.display(), err);
                                    continue;
                                }
                            };

                            let bytecode_object = match &metadata.bytecode {
                                Some(bytecode_object) => bytecode_object,
                                None => {
                                    log::error!("No bytecode found in {:?} {:?}", entry_path, &metadata.bytecode);
                                    continue;
                                }
                            };

                            // Convert the bytecode to a string, if it's 0x bytes make it '0x' as a string.
                            let bytecode = match bytecode_object.object.as_bytes() {
                                Some(bytecode) => bytecode.to_string(),
                                None => "0x".to_string() 
                            };

                            let deployed_bytecode_object = match &metadata.deployed_bytecode {
                                Some(deployed_bytecode_object) => deployed_bytecode_object,
                                None => {
                                    log::error!("No bytecode found in {:?} {:?}", entry_path, &metadata.deployed_bytecode);
                                    continue;
                                }
                            };

                            let deployed_bytecode = match &deployed_bytecode_object.bytecode {
                                Some(deployed_bytecode) => deployed_bytecode,
                                None => {
                                    log::error!("No deployed bytecode found in {:?} {:?}", entry_path, &metadata.deployed_bytecode);
                                    continue;
                                }
                            };

                            let dep_bytecode = match &deployed_bytecode.object.as_bytes() {
                                Some(bytecode) => bytecode.to_string(),
                                None => "0x".to_string()  
                            };

                            let kind = if bytecode == "0x" {
                                Kind::Interface
                            } else {
                                Kind::Contract
                            };

                            // Unwrap AST as it has the absolute_path in it
                            let ast = match &metadata.ast {
                                Some(ast) => ast,
                                None => {
                                    log::debug!("No ast found in {:?}", entry_path);
                                    continue; 
                                }
                            };

                            // Read the file bytes and store in file_contents 
                            let file_contents_path = Path::new(&repo_directory).join(&ast.absolute_path);
                            let file_contents = std::fs::read_to_string(file_contents_path).unwrap();

                            let contract = Contract {
                                contract_name: contract_name.to_owned(),
                                kind,
                                bytecode: bytecode.to_owned(),
                                deployed_bytecode: Some(dep_bytecode.to_owned()),
                                imports: None,
                                sourcemap: bytecode_object.source_map.to_owned(),
                                deployed_sourcemap: deployed_bytecode.source_map.to_owned(),
                                absolute_path: Some(ast.absolute_path.to_owned()),
                                id: metadata.id,
                                file_contents: Some(file_contents),
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
                    let metadata: ConfigurableContractArtifact = match ethers_solc::utils::read_json_file(entry_path) {
                        Ok(metadata) => metadata,
                        Err(err) => {
                            log::error!("Error reading JSON file '{}': {}", entry_path.display(), err);
                            continue;
                        }
                    };

                    // Get the current contract out of the map, iterate over the nodes
                    // And where there is an import grab that out of the map and append
                    // The imports to it.
                    if let Some(contract) = contract_map.get_mut(contract_name) {
                        for node in metadata.ast.unwrap().nodes {
                            if node.node_type == NodeType::ImportDirective {
                                let foreign_name = node.other.get("absolutePath").unwrap().to_string();
                                let foreign_name = Path::new(&foreign_name).file_stem().unwrap().to_str().unwrap();
                                // Get the imported contract (foreign_name) out of the map and append to the current contract imports.
                                if let Some(imported_contract) = contract_map_clone.get(foreign_name) {
                                    contract.imports.get_or_insert_with(Vec::new).push(imported_contract.clone());
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
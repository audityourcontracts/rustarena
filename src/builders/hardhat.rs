use std::process::{Command, exit};
use log;
use std::path::{Path};
use std::fs;
use std::collections::HashMap;
use walkdir::WalkDir;

use crate::builders::build::Build;
use crate::contract::{Contract, ContractKind};

use serde_derive::Deserialize;
use serde_derive::Serialize;

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    #[serde(rename = "_format")]
    pub format: String,
    pub contract_name: String,
    pub source_name: String,
    pub abi: Vec<Abi>,
    pub bytecode: String,
    pub deployed_bytecode: String,
    pub link_references: LinkReferences,
    pub deployed_link_references: DeployedLinkReferences,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Abi {
    pub inputs: Option<Vec<Input>>,
    pub state_mutability: Option<String>,
    #[serde(rename = "type")]
    pub type_field: String,
    pub anonymous: Option<bool>,
    pub name: Option<String>,
    #[serde(default)]
    pub outputs: Vec<Output>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Input {
    pub indexed: Option<bool>,
    pub internal_type: String,
    pub name: String,
    #[serde(rename = "type")]
    pub type_field: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    pub internal_type: String,
    pub name: String,
    #[serde(rename = "type")]
    pub type_field: String,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkReferences {
    #[serde(flatten)]
    pub contracts: HashMap<String, HashMap<String, Vec<Reference>>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Reference {
    pub length: u32,
    pub start: u32,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployedLinkReferences {
}

#[derive(Debug)]
pub enum HardhatMode {
    Yarn,
    Npm,
}

pub struct HardhatBuilder {
    pub mode: HardhatMode,
}

impl HardhatBuilder {
    pub fn new(mode: HardhatMode) -> Self {
        Self { mode }
    }

    pub fn set_mode(&mut self, mode: HardhatMode) {
        self.mode = mode;
    }

    pub fn flip_mode(&mut self) {
        match self.mode {
            HardhatMode::Yarn => self.mode = HardhatMode::Npm,
            HardhatMode::Npm => self.mode = HardhatMode::Yarn,
        }
    }
}

impl Build for HardhatBuilder {
    fn build(&self, directory: &str) -> Result<(String, Vec<Contract>), Box<dyn std::error::Error>> {
        let (install_cmd, install_arg, compile_cmd, compile_arg) = match self.mode {
            HardhatMode::Yarn => ("yarn", "", "yarn", "compile"),
            HardhatMode::Npm => ("npm", "install", "npx", "hardhat compile"),
        };

        log::info!("Executing {} in {}", install_cmd, directory);
        let install_result = Command::new(install_cmd)
            .arg(install_arg)
            .current_dir(directory)
            .output();
    
        if let Err(err) = install_result {
            log::error!("Error executing '{}' with '{}': {}", install_cmd, install_arg, err);
            exit(1);
        }
    
        // Execute `forge build` in the repository directory
        log::info!("Executing {} compile in {}", compile_cmd, directory);
        let build_result = Command::new(compile_cmd)
            .arg(compile_arg)
            .current_dir(directory)
            .output();
    
        if let Err(err) = build_result {
            log::error!("Error executing '{}' with '{}': {}", compile_cmd, compile_arg, err);
            exit(1);
        }

        let cache_dir = Path::new(&directory).join("cache");
        if !cache_dir.exists() || !cache_dir.is_dir() {
            log::error!("Error: 'cache' directory not found in {}", directory);
            //exit(1);
        }
        
        let artifacts_dir = Path::new(&directory).join("artifacts");
        log::info!("Checking for the artifacts directory {}", artifacts_dir.to_string_lossy());
        if !artifacts_dir.exists() {
            log::error!("Error: 'artifacts' directory {} not found in {}", artifacts_dir.to_string_lossy(), directory);
            //exit(1);
        }

        let result = process_artifacts_directory(directory);
        Ok(result)
    }
}

pub fn process_artifacts_directory(repo_directory: &str) -> (String, Vec<Contract>) {
    let out_dir = Path::new(&repo_directory).join("artifacts");
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
                            if !contract_name.ends_with(".dbg") { // Hardhat will add .dbg.json to some files. Ignoring those.
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
                    if !contract_name.ends_with(".dbg") { // Hardhat will add .dbg.json to some files. Ignoring those.
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

                        // LinkReferences contain the imports for a contract. If they aren't empty lets add them to the contract_map
                        if let Some(contract) = contract_map.get_mut(contract_name) {
                            for (_contract_path, inner_map) in &metadata.link_references.contracts {
                                for (contract_only, _references) in inner_map {
                                    // _contract_path is the path of the contract and contract_name.sol
                                    // contract_only is just the name of the contract, no path and no .sol
                                    let foreign_name = contract_only;
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
    }
    // contract_map should have all contracts with all imports
    let contracts: Vec<Contract> = contract_map.values().cloned().collect();
    (repo_directory.to_owned(), contracts)
}
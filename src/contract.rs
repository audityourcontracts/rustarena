use log;
use std::path::{Path};
use walkdir::WalkDir;
use crate::builders::build::Build;
use crate::builders::forge::ForgeBuilder;
use crate::builders::hardhat::{HardhatBuilder, HardhatMode};

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

pub fn process_repository(repo_directory: &str) -> Result<(String, Vec<Contract>), Box<dyn std::error::Error>> {
    for entry in WalkDir::new(repo_directory).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_dir() {
            let subdir = entry.path();
            let hardhat_config_ts = subdir.join("hardhat.config.ts");
            let hardhat_config_js = subdir.join("hardhat.config.js");

            if hardhat_config_ts.exists() || hardhat_config_js.exists() {
                let mut builder = HardhatBuilder::new(HardhatMode::Npm);

                if subdir.join("yarn.lock").exists() {
                    log::info!("Setting HardhatBuilder to Yarn mode");
                    builder.set_mode(HardhatMode::Yarn);
                }

                let (directory, contracts) = builder.build(subdir.to_str().unwrap())?;
                if contracts.is_empty() {
                    builder.flip_mode(); // Whatever mode you were that didn't work, try the other.
                    log::info!("No contracts found, trying HardhatBuilder in {:?} mode", builder.mode);
                    let (directory, contracts) = builder.build(subdir.to_str().unwrap())?;
                    if contracts.is_empty() {
                        log::info!("No contracts found, trying FoundryBuilder");
                        let foundry_builder = ForgeBuilder;
                        return foundry_builder.build(subdir.to_str().unwrap());
                    } else {
                        return Ok((directory, contracts));
                    }
                } else {
                    return Ok((directory, contracts));
                }
            } else {
                let foundry_file = subdir.join("foundry.toml");
                if foundry_file.exists() {
                    let builder = ForgeBuilder;
                    return builder.build(subdir.to_str().unwrap())
                }
            }
        }
    }
    Ok(("".to_string(), Vec::new()))
}
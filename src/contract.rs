use log;
use std::path::{Path};
use crate::builders::build::Build;
use crate::builders::forge::ForgeBuilder;

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
    let foundry_file = Path::new(repo_directory).join("foundry.toml");
    if !foundry_file.exists() {
        return Err("Error: 'foundry.toml' not found in the repository directory".into());
    }

    let builder = ForgeBuilder;
    builder.build(repo_directory)
}
use log;
use walkdir::WalkDir;
use crate::builders::build::Build;
use crate::builders::forge::ForgeBuilder;
use crate::builders::hardhat::{HardhatBuilder, HardhatMode};
use crate::builders::truffle::TruffleBuilder;
use crate::parsers::parse::Repo;

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

pub fn process_repository(repo: &Repo, keep_unsupported: bool) -> Result<(String, Vec<Contract>), Box<dyn std::error::Error>> {
    let repo_directory = &repo.name;
    // If we know how to build the repo but it doesn't work move to error
    let mut error_directory = String::from("repos/error");
    let repo_path = std::path::Path::new(repo_directory);
    if let Ok(repo_name) = repo_path.strip_prefix("repos") {
        if let Some(name) = repo_name.to_str() {
            error_directory.push('/');
            error_directory.push_str(name);
        } 
    }
    log::debug!("Error directory set to {}", error_directory);

    // We don't know how to build this kind of repo 
        let unsupported_base = format!("repos/unsupported/{}", &repo.parser);
        let mut unsupported_directory = String::from(unsupported_base);
        let repo_path = std::path::Path::new(repo_directory);
        if let Ok(repo_name) = repo_path.strip_prefix("repos") {
            if let Some(name) = repo_name.to_str() {
                unsupported_directory.push('/');
                unsupported_directory.push_str(name);
            } 
        }
    log::debug!("Unsupported directory set to {}", unsupported_directory);

    for entry in WalkDir::new(repo_directory).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_dir() {
            let subdir = entry.path();
            let hardhat_config_ts = subdir.join("hardhat.config.ts");
            let hardhat_config_js = subdir.join("hardhat.config.js");
            let foundry_file = subdir.join("foundry.toml");
            let truffle_file = subdir.join("truffle-config.js");

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
                        let builder = ForgeBuilder;
                        let (directory, contracts) = builder.build(subdir.to_str().unwrap())?;
                        if contracts.is_empty() {
                            log::error!("Attempted building with Hardhat then Foundry but failed. Moving {} to {}", directory, error_directory);
                            std::fs::create_dir_all(&error_directory)?;
                            std::fs::rename(repo_directory, &error_directory)?;
                            return Ok(("".to_string(), Vec::new())); 
                        }
                        return Ok((directory, contracts)) 
                    } else {
                        return Ok((directory, contracts))
                    }
                } else {
                    return Ok((directory, contracts))
                }
            } else if foundry_file.exists(){
                let builder = ForgeBuilder;
                let build_results = builder.build(subdir.to_str().unwrap())?;
                let (directory, contracts) = &build_results;
                if contracts.is_empty() {
                    log::error!("Attempted building with Foundry but failed. Moving repo {} to {}", directory, error_directory);
                    std::fs::create_dir_all(&error_directory)?;
                    std::fs::rename(repo_directory, &error_directory)?;
                    return Ok(("".to_string(), Vec::new()));
                }
                return Ok(build_results)
            } else if truffle_file.exists(){
                let builder = TruffleBuilder;
                let (directory, contracts) = builder.build(subdir.to_str().unwrap())?;
                if contracts.is_empty() {
                    log::error!("Attempted building with Truffle but failed. Moving {} to {}", directory, error_directory);
                    std::fs::create_dir_all(&error_directory)?;
                    std::fs::rename(repo_directory, &error_directory)?;
                    return Ok(("".to_string(), Vec::new()));
                }
                return Ok((directory, contracts))
            } else {
                if keep_unsupported {
                    log::error!("No buildable file found. Moving repo to {}", unsupported_directory);
                    std::fs::create_dir_all(&unsupported_directory)?;
                    std::fs::rename(repo_directory, &unsupported_directory)?;
                    return Ok(("".to_string(), Vec::new()));
                } else {
                    log::error!("No buildable file found. Deleting repo: {}", repo_directory);
                    std::fs::remove_dir_all(&repo_directory)?;
                }
            }
        }
    }
    // If none of the builders have returned we don't have anything.
    log::error!("No contracts returned from builders and we didn't exit earlier.");
    Ok(("".to_string(), Vec::new()))
}
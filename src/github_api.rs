use log;
use std::fs;

pub fn clone_repository(url: &str, directory: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Check if the directory exists
    if fs::metadata(directory).is_ok() {
        // Remove the directory if it exists
        fs::remove_dir_all(directory)?;
    }
    
    // Clone the repository
    log::info!("Cloning the repo {}", url);
    git2::Repository::clone(url, directory)?;
    
    Ok(())
}
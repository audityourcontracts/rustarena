use log;
use std::fs;
use git2::{Object, Oid, Repository};
use url::Url;

pub fn clone_repository(url: &str, directory: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Check if the directory exists
    if fs::metadata(directory).is_ok() {
        // Remove the directory if it exists
        fs::remove_dir_all(directory)?;
    }
    
    // Clone the repository
    log::info!("Cloning the repo {}", url);
    Repository::clone(url, directory)?;
    
    Ok(())
}

pub fn clone_repository_with_sha(link: &str, directory: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Check if the directory exists
    if fs::metadata(directory).is_ok() {
        // Remove the directory if it exists
        fs::remove_dir_all(directory)?;
    }

    if let Some((url, repo, commit)) = parse_github_url(link) {
        println!("URL to clone: {}", &url);
        println!("Repository: {}", &repo);
        println!("SHA Hash: {}", &commit);
        log::info!("Cloning the repo {}", repo);

        Repository::clone(&url, directory).unwrap();
        let repo: Repository = Repository::open(directory).unwrap();
        let obj: Object = repo.find_commit(Oid::from_str(&commit).unwrap()).unwrap().into_object();
        repo.checkout_tree(&obj, None).unwrap();
        repo.set_head_detached(obj.id()).unwrap();
    } else {
        println!("Invalid GitHub URL");
    }



    Ok(())
}


pub fn parse_github_url(url: &str) -> Option<(String, String, String)> {
    let parsed_url = Url::parse(url).ok()?;

    // Extract the repository URL
    let repository_url = parsed_url.join("..").map_err(|_| ()).ok()?.into();

    // Extract the repository path
    let path_segments = parsed_url.path_segments()?;
    let repository = path_segments.clone().take(2).collect::<Vec<_>>().join("/");

    // Extract the SHA hash from the branch or commit path segment
    let sha_hash = path_segments.last()?.to_string();

    Some((repository_url, repository, sha_hash))
}

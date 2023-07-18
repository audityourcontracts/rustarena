use log;
use std::fs;
use git2::{Oid, Repository};
use url::Url;

use crate::parsers::parse::Repo;

pub fn clone_repository(repository: &Repo) -> Result<(), Box<dyn std::error::Error>> {
    // Check if the directory exists
    if fs::metadata(&repository.name).is_ok() {
        // Remove the directory if it exists
        fs::remove_dir_all(&repository.name)?;
    }
    
    // Clone the repository
    if repository.commit.is_none() {
        log::info!("Cloning the repo {}", &repository.url);
        Repository::clone(&repository.url, &repository.name).map_err(|err| {
            err
        })?;
    } else {
        if let Some(commit) = &repository.commit {
            log::info!("Cloning the repo {} at commit {}", &repository.url, &commit);
            Repository::clone(&repository.url, &repository.name).map_err(|err| {
                err
            })?;
            let repo: Repository = Repository::open(&repository.name).unwrap();
            let obj = repo.find_commit(Oid::from_str(commit).unwrap()).unwrap().into_object();
            repo.checkout_tree(&obj, None).unwrap();
            repo.set_head_detached(obj.id()).unwrap();
        }
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

pub fn get_last_path_part(url: &str) -> Option<String> {
    if let Ok(parsed_url) = Url::parse(url) {
        parsed_url.path_segments()?.last().map(String::from)
    } else {
        None
    }
}

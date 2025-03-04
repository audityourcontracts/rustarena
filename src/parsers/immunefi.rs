use headless_chrome::{Browser, LaunchOptionsBuilder};
use log;
use scraper::{Html, Selector};
use url::Url;
use std::collections::HashSet;
use tokio::task::{spawn_blocking, spawn};
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::github_api;
use crate::parsers::parse::Repo;

pub struct ImmunefiParser {
    pub name: String,
    pub url: String,
}

impl ImmunefiParser {
    pub fn new() -> Self {
        ImmunefiParser {
            name: "immunefi".to_string(),
            url: "https://immunefi.com/explore/".to_string(),
        }
    }
}

impl ImmunefiParser {
    pub async fn parse_dom(&self) -> Result<Vec<Repo>, Box<dyn std::error::Error + Send + Sync>> {
        let mut repos: Vec<Repo> = Vec::new();
        let mut unique_github_links: HashSet<String> = HashSet::new();
        let url = self.url.clone(); 

        // Chrome needs to be run in blocking mode.
        let unique_links = spawn_blocking(move || {
            let inner_selector = Selector::parse("a").unwrap();
            let mut unique_bounty_links: HashSet<String> = HashSet::new();

            let launch_options= LaunchOptionsBuilder::default()
            .headless(true)  // Enable browser window if you need to test visually.
            .build()
            .expect("Failed to create browser instance");
    
            let browser = Browser::new(launch_options).expect("Failed to create browser");
            let tab = browser.new_tab().expect("Failed to create new tab");
            
            log::debug!("Immunefi parser navigating to {}", &url);
            tab.navigate_to(&url).unwrap();
        
            // Wait until navigation is completed
            match tab.wait_until_navigated() {
                Ok(_) => {
                    // Navigation completed successfully
                }
                Err(err) => {
                    log::error!("Error occurred during navigation: {}", err);
                }
            }
        
            // Wait for page load completion and grab the entire HTML.
            tab.wait_for_element("body").unwrap();

            let remote_object = tab
                .evaluate("document.documentElement.outerHTML", false)
                .ok().unwrap();
        
            let json = remote_object.value.unwrap();
            let html = json.as_str();
        
            let document = Html::parse_document(html.unwrap());

            for element in document.select(&inner_selector) {
                if let Some(link) = element.value().attr("href") {
                    // All immunefi bounty urls contain the word bounty
                    if link.contains("bounty") {
                        log::debug!("Found immunefi bounty link {}", link);
                        unique_bounty_links.insert(link.to_owned());
                    }
                }
            }
            unique_bounty_links
        }).await.unwrap();

        // For each bounty URL navigate to it and see if there's any github links in the page. 
        let base_url = "https://immunefi.com";
        let mut tasks = Vec::new();

        // Rate-limiting variables. Add max sessions to a semaphore so we don't overly hammer Immunefi.
        let max_concurrent_requests = 30; // Set the maximum number of concurrent requests
        let semaphore = Arc::new(Semaphore::new(max_concurrent_requests));

        for bounty_url in unique_links.into_iter().collect::<Vec<String>>() {
            let semaphore = Arc::clone(&semaphore); 

            // Spawn a task for each bounty. These run concurrently but all results are collected together. 
            let task = spawn(async move {
                let permit = semaphore.acquire().await.expect("Failed to acquire semaphore permit"); 
                let selector = Selector::parse("a").unwrap();
                let full_url = format!("{}{}", base_url, bounty_url);
                log::debug!("Parsing url {}", full_url);

                let response = reqwest::get(&full_url).await?;
                let body = response.text().await?;
                let document = Html::parse_document(&body);

                let mut github_links = HashSet::new();

                for element in document.select(&selector) {
                    if let Some(link) = element.value().attr("href") {
                        if link.contains("github.com") && !link.contains("immunefi-team") {
                            // Parse for the github repo format
                            let parsed_url = Url::parse(&link);
                            if let Ok(url) = parsed_url {
                                let path_segments = url.path_segments().unwrap();
                                let formatted_path = path_segments.take(2).collect::<Vec<_>>().join("/");
                                let formatted_url = format!("{}://{}/{}", url.scheme(), url.host_str().unwrap(), formatted_path);
                                github_links.insert(formatted_url.to_owned());
                                log::debug!("Found github url {}", link);
                            } else {
                                log::error!("Couldn't parse the url {}", link)
                            }
                        }
                    }
                }
                drop(permit);
                Ok::<HashSet<String>, Box<dyn std::error::Error + Send + Sync>>(github_links)
            });
            tasks.push(task);
        }

        // Wait for all tasks to complete (concurrently).
        let results = futures::future::try_join_all(tasks).await?;
        
        // Iterate over the results and collect the unique GitHub links
        for result in results {
            let github_links = result?;
            unique_github_links.extend(github_links);
        }
        
        // Turn the results into repos
        for github_link in unique_github_links {
            let parser = self.name.to_string();
            let url = github_link.to_string();
            let name = format!("repos/{}", github_api::get_last_path_part(&url.as_str()).unwrap());
            let commit = None;
            let repo = Repo { parser, url, name, commit };
            repos.push(repo);
        }
        log::info!("parser found {} repos", repos.len());
        Ok(repos)
    }
    
    fn url(&self) -> &str {
        &self.url
    }
}
use headless_chrome::{Browser, LaunchOptionsBuilder};
use log;
use scraper::{Html, Selector};

use crate::parsers::parse::Repo;
use crate::github_api;

pub struct Code4renaParser {
    pub name: String,
    pub url: String,
}

impl Code4renaParser {
    pub fn new() -> Self {
        Code4renaParser {
            name: "code4rena".to_string(),
            url: "https://code4rena.com/contests".to_string(),
        }
    }
}

impl Code4renaParser {
    pub async fn parse_dom(&self) -> Result<Vec<Repo>, Box<dyn std::error::Error + Send + Sync>>  {
        let launch_options= LaunchOptionsBuilder::default()
            .headless(true)  // Enable browser window
            .build()
            .expect("Failed to create browser instance");
    
        let browser = Browser::new(launch_options)?;
        let tab = browser.new_tab()?;
        tab.navigate_to(&self.url).unwrap();
    
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
        let selector = Selector::parse("a").unwrap();
    
        let mut repos: Vec<Repo> = Vec::new();
    
        for element in document.select(&selector) {
            if let Some(link) = element.value().attr("href") {
                if link.contains("github.com") && link != "https://github.com/code-423n4/" && link != "https://github.com/code-423n4/media-kit" {
                    log::debug!("Found github link {}", link);
                    let parser = self.name.to_string();
                    let url = link.to_string();
                    let name = format!("repos/{}", github_api::get_last_path_part(&url.as_str()).unwrap());
                    let commit = None;
                    let repo = Repo { parser, url, name, commit };
                    repos.push(repo);
                }
            }
        }
        log::info!("parser found {} repos", repos.len());
        Ok(repos)
    }
    
    fn url(&self) -> &str {
        &self.url
    }
}
use headless_chrome::{Browser, LaunchOptionsBuilder};
use log;
use scraper::{Html, Selector};
use url::Url;

use crate::parsers::parse::{WebsiteParser, Repo};

pub struct Code4renaParser {
    pub url: String,
}

impl Code4renaParser {
    pub fn new() -> Self {
        Code4renaParser {
            url: "https://code4rena.com/contests".to_string(),
        }
    }
}

impl WebsiteParser for Code4renaParser {
    fn parse_dom(&self) -> Result<Vec<Repo>, Box<dyn std::error::Error>>  {
        let launch_options= LaunchOptionsBuilder::default()
            .headless(true)  // Enable browser window
            .build()
            .expect("Failed to create browser instance");
    
        let browser = Browser::new(launch_options)?;
        let tab = browser.new_tab()?;
        tab.navigate_to(&self.url).unwrap();
    
        // Wait until navigation is completed
        tab.wait_until_navigated()?;
    
        // Wait for page load completion
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
                    log::info!("Found github link {}", link);
                    let url = link.to_string();
                    let name = format!("repos/{}", get_last_path_part(&url.as_str()).unwrap());
                    let commit = None;
                    let repo = Repo { url, name, commit };
                    repos.push(repo);
                }
            }
        }
        Ok(repos)
    }
    
    fn url(&self) -> &str {
        &self.url
    }
}

fn get_last_path_part(url: &str) -> Option<String> {
    if let Ok(parsed_url) = Url::parse(url) {
        parsed_url.path_segments()?.last().map(String::from)
    } else {
        None
    }
}
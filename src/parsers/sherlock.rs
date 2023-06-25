use reqwest;
use serde_json;
use markdown;
use scraper::{Html, Selector};
use crate::github_api;
use serde_derive::Deserialize;
use serde_derive::Serialize;
use crate::parsers::parse::{WebsiteParser, Repo};

pub struct SherlockParser{
    pub url: String,
}

impl SherlockParser{
    pub fn new() -> Self {
        SherlockParser{
            url: "https://mainnet-contest.sherlock.xyz/contests".to_string(),
        }
    }
}



pub type Root = Vec<Contests>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contests {
    #[serde(rename = "calc_completed")]
    pub calc_completed: bool,
    #[serde(rename = "ends_at")]
    pub ends_at: i64,
    #[serde(rename = "escalation_started_at")]
    pub escalation_started_at: Option<i64>,
    pub id: i64,
    #[serde(rename = "judging_ends_at")]
    pub judging_ends_at: i64,
    #[serde(rename = "judging_prize_pool")]
    pub judging_prize_pool: Option<i64>,
    #[serde(rename = "judging_repo_name")]
    pub judging_repo_name: String,
    #[serde(rename = "lead_judge_fixed_pay")]
    pub lead_judge_fixed_pay: Option<i64>,
    #[serde(rename = "lead_judge_handle")]
    pub lead_judge_handle: Option<String>,
    #[serde(rename = "lead_senior_auditor_fixed_pay")]
    pub lead_senior_auditor_fixed_pay: Option<i64>,
    #[serde(rename = "lead_senior_auditor_handle")]
    pub lead_senior_auditor_handle: Option<String>,
    #[serde(rename = "logo_url")]
    pub logo_url: String,
    pub private: bool,
    #[serde(rename = "prize_pool")]
    pub prize_pool: Option<i64>,
    pub rewards: Option<i64>,
    #[serde(rename = "score_sequence")]
    pub score_sequence: Option<i64>,
    #[serde(rename = "short_description")]
    pub short_description: String,
    #[serde(rename = "starts_at")]
    pub starts_at: Option<i64>,
    pub status: String,
    #[serde(rename = "template_repo_name")]
    pub template_repo_name: String,
    pub title: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contest {
    #[serde(rename = "description")]
    pub description: String,
}

impl WebsiteParser for SherlockParser {
    fn parse_dom(&self) -> Result<Vec<Repo>, Box<dyn std::error::Error>>  {
        let mut repos: Vec<Repo> = Vec::new();
        let response = reqwest::blocking::get(&self.url)?;

        if response.status().is_success() {
            let json_string = response.text()?; 
            let contests: Root = serde_json::from_str(&json_string)?;
            for contest in contests {
                if contest.status == "RUNNING" {
                    // Make a request to the specific contest URL
                    let contest_url = format!("{}/{}", self.url, contest.id);
                    let contest_response = reqwest::blocking::get(&contest_url)?;

                    if contest_response.status().is_success() {
                        let contest_data: Contest = serde_json::from_str(&contest_response.text()?)?;
                        let html : String = markdown::to_html(&contest_data.description);
                        // Process the contest data as needed
                        let document = Html::parse_document(&html);
                        let selector = Selector::parse("a").unwrap();

                        for element in document.select(&selector) {
                            if let Some(link) = element.value().attr("href") {
                                if link.contains("github.com") {
                                    // Parse the github url for repo and commit
                                    if let Some((url, repo, sha)) = github_api::parse_github_url(link) {
                                        log::debug!("Found github link {}. Cloning {} with sha {}", url, repo, sha);
                                        let name = format!("repos/{}", repo);
                                        let commit = Some(sha);
                                        let repo = Repo { url, name, commit};
                                        repos.push(repo);
                                    } else {
                                        log::info!("Invalid GitHub URL {}", link);
                                    } 
                                }
                            }
                        }
                    } else {
                        log::error!("Error parsing JSON for contest ID {}", contest.id);
                    }
                }
            }
        } else {
            log::error!("Error parsing JSON");
        }
        Ok(repos)
    }

    fn url(&self) -> &str {
        &self.url
    }
}
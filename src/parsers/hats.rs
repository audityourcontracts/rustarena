use crate::parsers::parse::{WebsiteParser, Repo};
use serde_json::json;
use tokio::runtime::Runtime;
use tokio::time::{Duration, sleep};
use serde_derive::Deserialize;
use serde_derive::Serialize;
use graphql_client;
use graphql_client::{GraphQLQuery, Response};
use std::collections::{HashMap, HashSet};
use url::Url;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hats {
    //pub version: Option<String>,
    //#[serde(rename = "project-metadata")]
    //pub project_metadata: ProjectMetadata,
    //pub source: Source,
    pub severities: Vec<Severity>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub name: String,
    pub url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMetadata {
    pub name: String,
    pub icon: String,
    pub token_icon: String,
    pub website: String,
    #[serde(rename = "type")]
    pub type_field: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Severity {
    pub name: String,
    #[serde(rename = "contracts-covered")]
    pub contracts_covered: Vec<HashMap<String, String>>,
    pub description: serde_json::Value,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "hats_schema.gql",
    query_path = "hats_query.graphql",
    response_derives = "Debug"
)]
pub struct MyQuery;
pub struct HatsParser{
    pub url: String,
}

impl HatsParser{
    pub fn new() -> Self {
        HatsParser{
            url: "https://api.thegraph.com/subgraphs/name/hats-finance/hats".to_string(),
        }
    }
}

impl WebsiteParser for HatsParser {
    fn parse_dom(&self) -> Result<Vec<Repo>, Box<dyn std::error::Error>>  {
        let mut repos: Vec<Repo> = Vec::new();

        let rt = Runtime::new().unwrap();

        // Construct the GraphQL query
        let variables: my_query::Variables = my_query::Variables {}; // Define your variables here
        let query = MyQuery::build_query(variables);

        // Create a Reqwest client
        let client = reqwest::blocking::Client::new();

        // Construct the GraphQL request body
        let body = json!({
            "query": query.query.to_string(),
            //"variables": variables,
        });

        // Send the GraphQL request
        let response = client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()?;

        let response_body: Response<my_query::ResponseData> = response.json()?;
        let base_url = "https://ipfs.io/ipfs";

        let mut unique_github_links: HashSet<String> = HashSet::new();

        if let Some(data) = response_body.data {
            for master in data.masters {
                if let Some(vaults) = master.vaults {
                    for vault in vaults {
                        log::debug!("Found vault {:?} with description hash {:?}", vault.id, vault.description_hash);
                        let ipfs_url = format!("{}/{}", base_url, vault.description_hash);
                        let response_result = reqwest::blocking::get(&ipfs_url);

                        match response_result {
                            Ok(response) => {
                                if response.status().is_success() {
                                    let ipfs_response: serde_json::Value = response.json()?;
                                    let hats: Hats = serde_json::from_value(ipfs_response)?;
                                    for severity in hats.severities {
                                        for contract_link in &severity.contracts_covered {
                                            for (_contract, link) in contract_link.iter() {
                                                if link.contains("github.com") {
                                                    // Parse for the github repo format
                                                    let parsed_url = Url::parse(&link);
                                                    if let Ok(url) = parsed_url {
                                                        let path_segments = url.path_segments().unwrap();
                                                        let formatted_path = path_segments.take(2).collect::<Vec<_>>().join("/");
                                                        let formatted_url = format!("{}://{}/{}", url.scheme(), url.host_str().unwrap(), formatted_path);
                                                        if unique_github_links.insert(formatted_url.to_owned()) {
                                                            // Only logging on new github urls
                                                            log::info!("Formatted github link: {}", formatted_url);
                                                        }
                                                    } else {
                                                        log::error!("Couldn't parse the url {}", link)
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } 
                            }
                            Err(err) => {
                                log::error!("Failed to send IPFS request: {}", err);
                            }
                        }
                    }
                }
            }
        }
        // Run the Tokio runtime
        rt.block_on(async {
            sleep(Duration::from_secs(1)).await;
        });

        for github_link in unique_github_links {
            let url = github_link.to_string();
            let name = format!("repos/{}", get_last_path_part(&url.as_str()).unwrap());
            let commit = None;
            let repo = Repo { url, name, commit };
            repos.push(repo);
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
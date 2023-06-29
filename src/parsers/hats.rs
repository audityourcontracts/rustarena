use crate::parsers::parse::Repo;
use crate::github_api;
use serde_json::json;
use serde_derive::Deserialize;
use serde_derive::Serialize;
use graphql_client;
use graphql_client::{GraphQLQuery, Response};
use std::collections::{HashMap, HashSet};
use url::Url;
use std::error::Error;
use std::time::Duration;

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
    schema_path = "src/parsers/graphql/hats_schema.gql",
    query_path = "src/parsers/graphql/hats_query.graphql",
    response_derives = "Debug"
)]
pub struct MyQuery;
pub struct HatsParser {
    pub urls: Vec<String>,
}

impl HatsParser {
    pub fn new() -> Self {
        HatsParser {
            urls: vec!["https://api.thegraph.com/subgraphs/name/hats-finance/hats".to_string(),
                       "https://api.thegraph.com/subgraphs/name/hats-finance/hats_polygon".to_string(),
                       "https://api.thegraph.com/subgraphs/name/hats-finance/hats_arbitrum".to_string(),
                       "https://api.thegraph.com/subgraphs/name/hats-finance/hats_optimism".to_string()],
        }
    }
}

impl HatsParser {
    pub async fn parse_dom(&self) -> Result<Vec<Repo>, Box<dyn Error + Send + Sync>>  {
        let mut repos: Vec<Repo> = Vec::new();

        // Construct the GraphQL query
        let variables: my_query::Variables = my_query::Variables {}; // Define your variables here
        let query = MyQuery::build_query(variables);

        // Create a Reqwest client
        let client = reqwest::Client::new();

        // Construct the GraphQL request body
        let body = json!({
            "query": query.query.to_string(),
        });

        let mut unique_github_links: HashSet<String> = HashSet::new();
        
        for url in &self.urls {
            log::info!("Querying {}", url);
            // Send the GraphQL request
            let response = client
                .post(url)
                .header("Content-Type", "application/json")
                .json(&body)
                .send().await?;

            let response_body: Response<my_query::ResponseData> = response.json().await?;
            let base_url = "https://ipfs.io/ipfs";

            if let Some(data) = response_body.data {
                for master in data.masters {
                    if let Some(vaults) = master.vaults {
                        for vault in vaults {
                            log::debug!("Found vault {:?} with description hash {:?}", vault.id, vault.description_hash);
                            let ipfs_url = format!("{}/{}", base_url, vault.description_hash);
                            let response_result = reqwest::Client::new()
                                                    .get(&ipfs_url).timeout(Duration::from_secs(3)).send().await;
                            match response_result {
                                Ok(response) => {
                                    if response.status().is_success() {
                                        let ipfs_response: serde_json::Value = response.json().await?;
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
                                                                log::info!("Found github repo: {}", formatted_url);
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
        }

        for github_link in unique_github_links {
            let url = github_link.to_string();
            let name = format!("repos/{}", github_api::get_last_path_part(&url.as_str()).unwrap());
            let commit = None;
            let repo = Repo { url, name, commit };
            log::debug!("Adding repo {:?}", repo);
            repos.push(repo);
        }
        Ok(repos)
    }

    fn url(&self) -> &str {
        // Just return the first url if asked.
        &self.urls[0]
    }
}
#[derive(Debug)]
pub struct Repo {
    pub url: String,
    pub name: String,
    pub commit: Option<String>,
}

pub trait WebsiteParser {
    fn parse_dom(&self) -> Result<Vec<Repo>, Box<dyn std::error::Error>>;
    fn url(&self) -> &str;
}
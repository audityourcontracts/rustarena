pub struct Repo {
    pub url: String,
    pub name: String
}

pub trait WebsiteParser {
    fn parse_dom(&self, website_url: &str) -> Result<Vec<Repo>, Box<dyn std::error::Error>>;
}
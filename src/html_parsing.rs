pub struct Repo {
    pub url: String,
    pub name: String
}

pub trait WebsiteParser {
    fn parse_dom(&self) -> Result<Vec<Repo>, Box<dyn std::error::Error>>;
}
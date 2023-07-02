use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct Repo {
    pub parser: String,
    pub url: String,
    pub name: String,
    pub commit: Option<String>,
}

#[derive(Debug)]
pub struct ParseError {
    message: String,
}

impl ParseError {
    pub fn new(message: &str) -> Self {
        ParseError {
            message: message.to_string(),
        }
    }
}

impl Error for ParseError {}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
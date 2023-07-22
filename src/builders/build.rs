use crate::contract::Contract;

pub trait Build {
    fn build(&self, directory: &str) -> Result<(String, Vec<Contract>), Box<dyn std::error::Error>>;
}
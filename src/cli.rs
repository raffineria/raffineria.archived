use failure::Error;
use std::marker::Sized;

pub trait Cli: Sized {
    fn use_dotenv(&self) -> bool {
        false
    }
    fn required_env_vars() -> Vec<String> {
        Vec::new()
    }

    fn name() -> &'static str;
    fn run(self) -> Result<(), Error>;
}

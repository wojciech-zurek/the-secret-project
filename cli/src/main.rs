use crate::error::{CliError, ErrorType};

mod error;
mod cli;
mod reader;
mod write;
mod process;

fn main() -> Result<(), CliError> {
    let matches = cli::build();
    process::execute(&matches)
}

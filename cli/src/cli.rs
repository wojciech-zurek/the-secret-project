use clap::{Arg, ArgMatches, Command};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");
const NAME: &str = env!("CARGO_PKG_NAME");

pub fn build() -> ArgMatches {
    Command::new(NAME)
        .about("Simple CSV reader for transaction analyze")
        .version(VERSION)
        .arg_required_else_help(true)
        .author(AUTHOR)
        .arg(Arg::new("file_path")
            .help("File path where csv file is located")
            .required(true)
            .index(1)
        ).get_matches()
}

use std::str::FromStr;

use clap::{AppSettings, Clap};
use indoc::indoc;

#[derive(Debug)]
enum Filter {
    Builds,
    Artifacts,
    Messages,
}

impl FromStr for Filter {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "builds" => Self::Builds,
            "messages" => Self::Messages,
            "artifacts" => Self::Artifacts,
            _ => Err(format!("{} does not match any filtereable event", s))?
        })
    }
}

/// This doc string acts as a help message when the user runs '--help'
/// as do all doc strings on fields
#[derive(Debug, Clap)]
#[clap(version = "0.1.0", author = "Mathias Pius <contact@pius.io>")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    input: Option<String>,
    #[clap(short, long, about = indoc! {"
        If set, aggregates the output into a single document containing
        all the information from the log, instead of outputting individual
        events as they happen.        
    "})]
    aggregate: bool,
    #[clap(short, long, about = indoc! {"
        Hello world
    "})]
    filter: Vec<Filter>,
}

fn main() {
    let opts = Opts::parse();

}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use clap::Clap;
    use crate::Opts;

    #[test]
    fn test_parsing() {
        let err = Opts::try_parse_from(&["bin-name", "-f", "unknown_event"]).expect_err("filtering by unknown_event should error");
        assert_eq!(err.source().unwrap().to_string(), "unknown_event does not match any filtereable event");
    }
}

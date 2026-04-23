pub mod handlers;
pub mod output;
pub mod parser;

use clap::Parser;

pub use parser::TodoCli;

pub fn parse(args: &[String]) -> Result<TodoCli, clap::Error> {
    TodoCli::try_parse_from(std::iter::once("todo".to_owned()).chain(args.iter().cloned()))
}

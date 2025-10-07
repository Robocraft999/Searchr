mod engine;
mod index;

use std::path::PathBuf;
use clap::{Parser, Subcommand};
use crate::engine::Engine;
use crate::index::{Index, LocalFilesystemSource};

#[derive(Parser, Debug)]
#[command(name = "searchr")]
#[command(about = "A local search engine", long_about = None, author = "Robocraft999")]
struct Cli {
    #[command(subcommand)]
    command: Commands
}

#[derive(Subcommand, Debug)]
enum Commands {
    Index,
    #[command(arg_required_else_help = true, aliases = ["se"])]
    Search {
        #[arg(value_name = "QUERY")]
        query: String,
    }
}

fn main() {
    let mut index = Index::new();
    index.add(Box::from(LocalFilesystemSource::new("./test")));
    let mut engine = Engine::new(index);

    let args = Cli::parse();
    match args.command {
        Commands::Index => {
            engine.index();
        }
        Commands::Search {query} => {
            println!("Searching for {}", query);
        }
    }
    println!("Finished");
}

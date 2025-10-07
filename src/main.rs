mod engine;
mod index;

use crate::engine::Engine;
use crate::index::{Index, LocalFilesystemSource};
use clap::{Parser, Subcommand};

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
    index.add(Box::from(LocalFilesystemSource::new("./test/sub")));
    index.add(Box::from(LocalFilesystemSource::new("C:/Users/Admin/.rustup/toolchains/stable-x86_64-pc-windows-msvc/share/doc/rust/html/std/collections")));
    let mut engine = Engine::new(index);

    let args = Cli::parse();
    match args.command {
        Commands::Index => {
            engine.index();
        }
        Commands::Search {query} => {
            engine.search(query);
        }
    }
    println!("Finished");
}

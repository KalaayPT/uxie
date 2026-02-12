use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod cli;

use crate::cli::commands::{
    cmd_egg_moves, cmd_encounter, cmd_event, cmd_evolution, cmd_header, cmd_item, cmd_learnset,
    cmd_map, cmd_move, cmd_parse_header, cmd_personal, cmd_resolve_script, cmd_trainer,
};

#[derive(Parser)]
#[command(name = "uxie")]
#[command(author = "Kalaay")]
#[command(version)]
#[command(about = "Data fetching tool for Pokemon Gen 4 Romhacking", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Args)]
struct ProjectArgs {
    #[arg(short, long, default_value = ".")]
    project: PathBuf,

    #[arg(short, long)]
    decomp: Option<PathBuf>,

    #[arg(long)]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    Header {
        #[arg(default_value = ".")]
        path: PathBuf,

        #[arg(long)]
        json: bool,
    },

    Map {
        id: u16,

        #[command(flatten)]
        args: ProjectArgs,
    },

    Event {
        id: u32,

        #[command(flatten)]
        args: ProjectArgs,
    },

    Encounter {
        id: u32,

        #[command(flatten)]
        args: ProjectArgs,
    },

    Symbols {
        path: PathBuf,

        #[arg(long)]
        only_defines: bool,

        #[arg(long)]
        only_enums: bool,

        #[arg(long)]
        json: bool,
    },

    ResolveScript {
        path: PathBuf,

        #[arg(short, long, default_value = ".")]
        decomp: PathBuf,
    },

    Personal {
        id: String,

        #[command(flatten)]
        args: ProjectArgs,
    },

    Move {
        id: String,

        #[command(flatten)]
        args: ProjectArgs,
    },

    Item {
        id: String,

        #[command(flatten)]
        args: ProjectArgs,
    },

    Trainer {
        id: u16,

        #[command(flatten)]
        args: ProjectArgs,
    },

    Evolution {
        id: String,

        #[command(flatten)]
        args: ProjectArgs,
    },

    Learnset {
        id: String,

        #[command(flatten)]
        args: ProjectArgs,
    },

    EggMoves {
        species: Option<String>,

        #[command(flatten)]
        args: ProjectArgs,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Header { path, json } => cmd_header(&path, json),
        Commands::Map { id, args } => cmd_map(id, &args.project, args.decomp, args.json),
        Commands::Event { id, args } => cmd_event(id, &args.project, args.decomp, args.json),
        Commands::Encounter { id, args } => {
            cmd_encounter(id, &args.project, args.decomp, args.json)
        }
        Commands::Symbols {
            path,
            only_defines,
            only_enums,
            json,
        } => cmd_parse_header(&path, only_defines, only_enums, json),
        Commands::ResolveScript { path, decomp } => cmd_resolve_script(&path, &decomp),
        Commands::Personal { id, args } => cmd_personal(&id, &args.project, args.decomp, args.json),
        Commands::Move { id, args } => cmd_move(&id, &args.project, args.decomp, args.json),
        Commands::Item { id, args } => cmd_item(&id, &args.project, args.decomp, args.json),
        Commands::Trainer { id, args } => cmd_trainer(id, &args.project, args.decomp, args.json),
        Commands::Evolution { id, args } => {
            cmd_evolution(&id, &args.project, args.decomp, args.json)
        }
        Commands::Learnset { id, args } => cmd_learnset(&id, &args.project, args.decomp, args.json),
        Commands::EggMoves { species, args } => {
            cmd_egg_moves(species, &args.project, args.decomp, args.json)
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

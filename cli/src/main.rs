use std::path::PathBuf;

use clap::{Parser, Subcommand};
use quetzalcoatl_core::config::Config;
use quetzalcoatl_core::ops;
use quetzalcoatl_core::xochitl::TreeNode;

#[derive(Parser)]
#[command(name = "quetzalcoatl", about = "Reliable file import for the reMarkable 2")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Upload a local PDF onto the tablet.
    Import {
        file: PathBuf,
        /// Target folder path, `//`-separated. Missing folders are created
        /// automatically. Defaults to the tablet's root.
        #[arg(long)]
        folder: Option<String>,
        #[arg(long, default_value = "quetzalcoatl.toml")]
        config: PathBuf,
        /// Re-download the uploaded file and compare its hash to the local
        /// one. Roughly doubles transfer time; off by default.
        #[arg(long)]
        verify_hash: bool,
        /// Skip restarting xochitl after upload.
        #[arg(long)]
        no_restart: bool,
    },
    /// Move an existing document or folder to a different folder path.
    Move {
        /// UUID of the document or folder to move.
        uuid: String,
        #[arg(long = "to")]
        to: String,
        #[arg(long, default_value = "quetzalcoatl.toml")]
        config: PathBuf,
    },
    /// Print the tablet's current folder/document tree.
    ListFolders {
        #[arg(long, default_value = "quetzalcoatl.toml")]
        config: PathBuf,
    },
    /// Restart xochitl and wait for it to report `active`.
    RestartXochitl {
        #[arg(long, default_value = "quetzalcoatl.toml")]
        config: PathBuf,
    },
    /// Connect and disconnect, to validate a config without importing anything.
    TestConnection {
        #[arg(long, default_value = "quetzalcoatl.toml")]
        config: PathBuf,
    },
}

fn print_tree(node: &TreeNode, depth: usize) {
    let indent = "  ".repeat(depth);
    if depth == 0 {
        println!("{indent}/ (root)");
    } else {
        println!("{indent}{}/  [{}]", node.name, node.uuid);
    }
    for (uuid, name) in &node.documents {
        println!("{indent}  {name}  [{uuid}]");
    }
    for folder in &node.folders {
        print_tree(folder, depth + 1);
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Import {
            file,
            folder,
            config,
            verify_hash,
            no_restart,
        } => {
            let config = Config::load(&config)?;
            let transport = ops::connect(&config).await?;
            let options = ops::ImportOptions {
                target_folder: folder.unwrap_or(config.default_folder.clone()),
                verify_hash,
                restart: !no_restart,
            };
            let uuid = ops::import_file(&transport, &file, &options).await?;
            println!("Imported {} as {uuid}", file.display());
        }
        Command::Move { uuid, to, config } => {
            let config = Config::load(&config)?;
            let transport = ops::connect(&config).await?;
            ops::move_entry(&transport, &uuid, &to).await?;
            println!("Moved {uuid} to \"{to}\"");
        }
        Command::ListFolders { config } => {
            let config = Config::load(&config)?;
            let transport = ops::connect(&config).await?;
            let tree = ops::list_tree(&transport).await?;
            print_tree(&tree, 0);
        }
        Command::RestartXochitl { config } => {
            let config = Config::load(&config)?;
            let transport = ops::connect(&config).await?;
            ops::restart_xochitl(&transport).await?;
            println!("xochitl restarted and reports active");
        }
        Command::TestConnection { config } => {
            let config = Config::load(&config)?;
            ops::test_connection(&config).await?;
            println!("Connected to {}@{} successfully", config.username, config.host);
        }
    }

    Ok(())
}

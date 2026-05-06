use anyhow::Result;
use clap::{Parser, Subcommand};
use mdtool_core::domain::policy::FilesystemPolicy;
use mdtool_core::domain::selectors::{BlockSelector, InsertPosition};
use mdtool_core::services::block_service::BlockService;

#[derive(Parser)]
#[command(name = "mdtool", version, about = "Markdown operations engine")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show heading outline
    Outline {
        file: String,
        #[arg(long, default_value = "3")]
        max_depth: u8,
    },
    /// Read block data
    ReadBlock {
        file: String,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        id: Option<u32>,
        #[arg(long, default_value = "data")]
        view: String,
    },
    /// Read blocks by type
    ReadBlocks {
        file: String,
        #[arg(long)]
        r#type: String,
    },
    /// Search for text
    Search {
        file: String,
        query: String,
        #[arg(long)]
        case_sensitive: bool,
    },
    /// Edit operations
    #[command(subcommand)]
    Edit(EditCommand),
    /// Validate document
    Validate { file: String },
    /// Normalize formatting
    Normalize {
        file: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Format ASCII art
    FormatAscii {
        file: String,
        #[arg(long, default_value = "format_only")]
        mode: String,
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum EditCommand {
    Replace {
        file: String,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        id: Option<u32>,
        #[arg(long)]
        content: String,
        #[arg(long)]
        dry_run: bool,
    },
    Insert {
        file: String,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        id: Option<u32>,
        #[arg(long)]
        after: bool,
        #[arg(long)]
        before: bool,
        #[arg(long)]
        content: String,
        #[arg(long)]
        dry_run: bool,
    },
    Delete {
        file: String,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        id: Option<u32>,
        #[arg(long)]
        dry_run: bool,
    },
    RenameSection {
        file: String,
        #[arg(long)]
        path: String,
        #[arg(long)]
        new_title: String,
        #[arg(long)]
        dry_run: bool,
    },
    ToggleTask {
        file: String,
        #[arg(long)]
        id: u32,
        #[arg(long)]
        dry_run: bool,
    },
}

fn make_selector(path: Option<String>, id: Option<u32>) -> BlockSelector {
    if let Some(id) = id {
        BlockSelector::from_id(mdtool_core::primitives::BlockId(id))
    } else if let Some(path) = path {
        BlockSelector::from_path(&path)
    } else {
        BlockSelector::default()
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let policy = FilesystemPolicy::default();
    let svc = BlockService::new(policy);

    match cli.command {
        Commands::Outline { file, max_depth } => {
            let result = svc.read_outline(&file, max_depth, true)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::ReadBlock { file, path, id, view: _ } => {
            let selector = make_selector(path, id);
            let result = svc.read_block(&file, &selector, true)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::ReadBlocks { file, r#type } => {
            let result = svc.read_blocks_by_type(&file, &r#type)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::Search { file, query, case_sensitive } => {
            let result = svc.search_blocks(&file, &query, None, case_sensitive)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::Validate { file } => {
            let result = svc.validate(&file)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::Normalize { file, dry_run } => {
            let result = svc.normalize(&file, None, dry_run)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::FormatAscii { file, mode: _, dry_run } => {
            let ascii_svc = mdtool_core::services::ascii_service::AsciiService::new(FilesystemPolicy::default());
            let result = ascii_svc.format_ascii(&file, mdtool_core::ascii::model::AsciiMode::default(), dry_run)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::Edit(edit_cmd) => match edit_cmd {
            EditCommand::Replace { file, path, id, content, dry_run } => {
                let selector = make_selector(path, id);
                let result = svc.replace_block(&file, &selector, &content, dry_run)?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            EditCommand::Insert { file, path, id, after, before: _, content, dry_run } => {
                let selector = make_selector(path, id);
                let position = if after { InsertPosition::After } else { InsertPosition::Before };
                let result = svc.insert_block(&file, &selector, &content, position, dry_run)?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            EditCommand::Delete { file, path, id, dry_run } => {
                let selector = make_selector(path, id);
                let result = svc.delete_block(&file, &selector, dry_run)?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            EditCommand::RenameSection { file, path, new_title, dry_run } => {
                let selector = BlockSelector::from_path(&path);
                let result = svc.rename_section(&file, &selector, &new_title, dry_run)?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            EditCommand::ToggleTask { file, id, dry_run } => {
                let selector = BlockSelector::from_id(mdtool_core::primitives::BlockId(id));
                let result = svc.toggle_task(&file, &selector, dry_run)?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        },
    }

    Ok(())
}

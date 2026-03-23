mod cmd_index;
mod cmd_search;
mod cmd_status;
mod output;
mod regex_decompose;

use std::path::PathBuf;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "trigrep",
    about = "Indexed regex search for large codebases",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build or rebuild the trigram index
    Index {
        /// Directory to index (default: current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Force rebuild even if index exists
        #[arg(long)]
        force: bool,
    },
    /// Search for a regex pattern using the index
    Search {
        /// Regex pattern to search for
        pattern: String,

        /// Directory to search (default: current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Case-insensitive matching
        #[arg(short = 'i', long)]
        ignore_case: bool,

        /// Print only count of matching lines per file
        #[arg(short = 'c', long)]
        count: bool,

        /// Print only filenames with matches
        #[arg(short = 'l', long)]
        files_with_matches: bool,

        /// Show line numbers (default: on)
        #[arg(short = 'n', long, default_value_t = true)]
        line_number: bool,

        /// Surround pattern with word boundaries
        #[arg(short = 'w', long)]
        word_regexp: bool,

        /// Output results as JSON
        #[arg(long)]
        json: bool,

        /// Skip index, grep all files directly
        #[arg(long)]
        no_index: bool,

        /// Print query statistics
        #[arg(long)]
        stats: bool,

        /// Lines of context before match
        #[arg(short = 'B', long, default_value_t = 0)]
        before_context: usize,

        /// Lines of context after match
        #[arg(short = 'A', long, default_value_t = 0)]
        after_context: usize,

        /// Lines of context before and after match
        #[arg(short = 'C', long, default_value_t = 0)]
        context: usize,
    },
    /// Show index status and metadata
    Status {
        /// Directory to check (default: current directory)
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Index { path, force: _ } => cmd_index::run(&path),
        Commands::Search {
            pattern,
            path,
            ignore_case,
            count,
            files_with_matches,
            line_number,
            word_regexp,
            json,
            no_index,
            stats,
            before_context,
            after_context,
            context,
        } => {
            let ctx = if context > 0 { context } else { 0 };
            let opts = cmd_search::SearchOptions {
                pattern,
                case_insensitive: ignore_case,
                count_only: count,
                files_only: files_with_matches,
                line_numbers: line_number,
                word_boundary: word_regexp,
                json,
                no_index,
                stats,
                context_before: if before_context > 0 { before_context } else { ctx },
                context_after: if after_context > 0 { after_context } else { ctx },
            };
            cmd_search::run(&opts, &path)
        }
        Commands::Status { path } => cmd_status::run(&path),
    };

    if let Err(e) = result {
        eprintln!("error: {:#}", e);
        std::process::exit(1);
    }
}

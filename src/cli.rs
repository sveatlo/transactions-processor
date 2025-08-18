use clap::Parser;
use std::path::PathBuf;

fn short_version() -> &'static str {
    let short_version = crate::VERSION.to_string();

    Box::leak(short_version.into_boxed_str())
}

fn long_version() -> &'static str {
    let long_version = format!(
        "{} ({}, {})",
        crate::VERSION,
        crate::GIT_HASH,
        crate::BUILD_TIMESTAMP
    );

    Box::leak(long_version.into_boxed_str())
}

#[derive(Parser, Debug, Clone)]
#[command(name = "transactions-processor")]
#[command(author)]
#[command(version = short_version())]
#[command(long_version = long_version())]
#[command(max_term_width = 120)]
#[command(
    help_expected = true,
    disable_help_subcommand = true,
    infer_subcommands = true
)]
pub struct Cli {
    #[clap(
        value_name = "TRANSACTIONS_FILE",
        index = 1,
        help = "Path to CSV file containing the transactions to process"
    )]
    pub transactions_file: PathBuf,
}

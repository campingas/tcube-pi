use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use tcube_pi::metrics::latency::MeasureConfig;

#[derive(Debug, Parser)]
#[command(about = "Measure T-Cube Pi admin impact on button latency")]
struct Cli {
    #[arg(long, default_value = "http://127.0.0.1:8080")]
    base_url: String,

    #[arg(long, default_value = "content/content.json")]
    content: PathBuf,

    #[arg(long, default_value_t = 1_000)]
    button_presses: usize,

    #[arg(long, default_value_t = 600)]
    admin_requests: usize,

    #[arg(long, default_value_t = 4)]
    admin_workers: usize,

    #[arg(long)]
    database: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    tcube_pi::metrics::latency::run(MeasureConfig {
        base_url: cli.base_url,
        content: cli.content,
        button_presses: cli.button_presses,
        admin_requests: cli.admin_requests,
        admin_workers: cli.admin_workers,
        database: cli.database,
    })
}

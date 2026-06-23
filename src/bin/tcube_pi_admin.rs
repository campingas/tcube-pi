use std::path::PathBuf;

use clap::Parser;
use tcube_pi::config::AdminConfig;

#[derive(Debug, Parser)]
#[command(about = "T-Cube Pi-hosted admin service")]
struct Cli {
    #[arg(long, default_value = "127.0.0.1:8080")]
    bind: String,

    #[arg(long, default_value = "data/tcube.sqlite3")]
    database: PathBuf,

    #[arg(long, default_value = "admin-ui")]
    ui_dist: PathBuf,

    #[arg(long, default_value = "data/media")]
    media_root: PathBuf,

    #[arg(long, default_value = "content")]
    content_root: PathBuf,

    #[arg(long, default_value = "tcube.local")]
    hostname: String,

    #[arg(long, default_value = "10.55.0.1")]
    usb_address: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    tcube_pi::server::run(AdminConfig {
        bind: cli.bind,
        database: cli.database,
        ui_dist: cli.ui_dist,
        media_root: cli.media_root,
        content_root: cli.content_root,
        hostname: cli.hostname,
        usb_address: cli.usb_address,
    })
    .await
}

use clap::Parser;

use tcube_pi::config::DeviceApiConfig;

#[derive(Debug, Parser)]
#[command(about = "T-Cube device content sync API")]
struct Cli {
    #[arg(long, default_value = "127.0.0.1:8081")]
    bind: String,

    #[arg(long, default_value = "data/tcube.sqlite3")]
    database: std::path::PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    tcube_pi::device_api::run(DeviceApiConfig {
        bind: cli.bind,
        database: cli.database,
    })
    .await
}

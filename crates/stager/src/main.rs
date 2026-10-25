mod stage;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Replay committed shielded transaction fixtures into darksidewalletd")]
struct Args {
    #[arg(long, default_value = "http://127.0.0.1:9067")]
    endpoint: String,
    #[arg(long, default_value = "fixtures")]
    fixtures: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    stage::stage(&args.endpoint, &args.fixtures).await
}

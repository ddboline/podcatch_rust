use anyhow::Error;

use podcatch_rust::podcatch_opts::PodcatchOpts;

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::init();
    tokio::spawn(async move {
        PodcatchOpts::process_args().await
    }).await.unwrap()
}

use anyhow::Error;

use podcatch_rust::podcatch_opts::PodcatchOpts;

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::init();
    PodcatchOpts::process_args().await
}

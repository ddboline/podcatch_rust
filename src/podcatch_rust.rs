use anyhow::Error;

use podcatch_rust::podcatch_opts::PodcatchOpts;

fn main() -> Result<(), Error> {
    env_logger::init();
    PodcatchOpts::process_args()
}

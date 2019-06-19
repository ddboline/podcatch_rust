use failure::Error;

use podcatch_rust::podcatch_opts::PodcatchOpts;

fn main() -> Result<(), Error> {
    PodcatchOpts::process_args()
}

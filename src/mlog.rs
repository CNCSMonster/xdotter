use crate::RunArgs;

pub fn init_logger(args: &RunArgs) -> anyhow::Result<()> {
    let verbose = args.verbose;
    let quiet = args.quiet;
    if verbose && !quiet {
        std::env::set_var("RUST_LOG", "trace");
    } else if quiet {
        std::env::set_var("RUST_LOG", "error");
    } else {
        let level = std::env::var("RUST_LOG");
        if level.is_err() {
            std::env::set_var("RUST_LOG", "info");
        }
    }
    env_logger::init();

    Ok(())
}

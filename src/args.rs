pub fn get_dry_run(args: &clap::ArgMatches) -> bool {
    args.get_flag("dry-run")
}
pub fn get_quiet(args: &clap::ArgMatches) -> bool {
    args.get_flag("quiet")
}

pub fn get_verbose(args: &clap::ArgMatches) -> bool {
    args.get_flag("verbose")
}

pub fn get_interactive(args: &clap::ArgMatches) -> bool {
    args.get_flag("interactive")
}

pub fn get_start_config(args: &clap::ArgMatches) -> String {
    args.get_one::<String>("config")
        .cloned()
        .unwrap_or("xdotter.toml".to_string())
}

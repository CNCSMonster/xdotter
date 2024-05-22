pub fn get_dry_run(args: &clap::ArgMatches) -> bool {
    args.get_flag("dry-run")
}

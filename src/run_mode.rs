use log::info;

use crate::RunArgs;

static mut DRY_RUN_FLAG: bool = false;
static mut INTERACTIVE_FLAG: bool = false;
static mut FORCE_FLAG: bool = false;

pub fn init_run_mode(args: &RunArgs) -> anyhow::Result<()> {
    if args.dry_run {
        info!("running in dry-run mode");
    }
    if args.interactive {
        info!("running in interactive mode");
    }
    if args.force {
        info!("running in force mode");
    }
    set_dry_run_mode(args.dry_run);
    set_interactive_mode(args.interactive);
    set_force_mode(args.force);
    Ok(())
}

pub fn set_dry_run_mode(flag: bool) {
    unsafe {
        DRY_RUN_FLAG = flag;
    }
}

pub fn set_interactive_mode(flag: bool) {
    unsafe {
        INTERACTIVE_FLAG = flag;
    }
}

pub fn set_force_mode(flag: bool) {
    unsafe {
        FORCE_FLAG = flag;
    }
}

pub fn on_dry_run_mode() -> bool {
    unsafe { DRY_RUN_FLAG }
}

pub fn on_interactive_mod() -> bool {
    unsafe { INTERACTIVE_FLAG }
}

pub fn on_force_mode() -> bool {
    unsafe { FORCE_FLAG }
}

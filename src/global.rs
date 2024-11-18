static mut DRY_RUN_FLAG: bool = false;
static mut INTERACTIVE_FLAG: bool = false;
static mut FORCE_FLAG: bool = false;

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

use crate::error::XdError;

pub fn run() -> Result<(), XdError> {
    println!("xdotter {}", env!("CARGO_PKG_VERSION"));
    Ok(())
}

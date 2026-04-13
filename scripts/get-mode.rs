use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: get-mode <filepath>");
        std::process::exit(1);
    }

    let path = &args[1];
    match fs::metadata(path) {
        Ok(metadata) => {
            let mode = metadata.permissions().mode() & 0o777;
            println!("{:o}", mode);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

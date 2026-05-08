use crate::error::XdError;

const BASH: &str = include_str!(concat!(env!("OUT_DIR"), "/xd.bash"));
const ZSH: &str = include_str!(concat!(env!("OUT_DIR"), "/_xd"));
const FISH: &str = include_str!(concat!(env!("OUT_DIR"), "/xd.fish"));

pub fn run(shell: &str) -> Result<(), XdError> {
    let s = match shell.to_ascii_lowercase().as_str() {
        "bash" => BASH,
        "zsh" => ZSH,
        "fish" => FISH,
        other => {
            return Err(XdError::cli(format!(
                "不支持的 shell: {}. 支持: bash, zsh, fish",
                other
            )));
        }
    };
    print!("{}", s);
    Ok(())
}

use log::{error, info};
use maplit::hashmap;

use crate::Config;

pub fn new() -> anyhow::Result<()> {
    let config = Config {
        dependencies: Some(hashmap! {
            "go".to_string() => "testdata/go".to_string(),
        }),
        links: Some(hashmap! {
            "testdata/mm".to_string() => "~/.cache/mm".to_string(),
        }),
    };
    let config_str = toml::to_string(&config).unwrap();
    info!("creating xdotter.toml");
    std::fs::write("xdotter.toml", config_str).unwrap_or_else(|e| {
        error!("failed to create xdotter.toml: {}", e);
    });
    info!("Created xdotter.toml");
    Ok(())
}

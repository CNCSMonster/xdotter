use anyhow::Error;
use std::{
    fs,
    os::unix::fs::symlink,
    path::{self, Path},
};

// 创建路径为link的软链接到actual_path
pub fn create_link(actual_path: &str, link: &str) -> Result<(), Error> {
    // 获取actual_path的绝对路径
    let actual_path = std::fs::canonicalize(actual_path).unwrap();
    // 获取link的绝对路径
    let home_dir = dirs::home_dir().unwrap();
    let link = link.replace("~", home_dir.to_str().unwrap());
    println!("link: {}", link);
    // 化简路径
    let link = Path::new(&link);
    // 获取link的目录,保证link的目录存在
    let link_dir = link.parent().unwrap();
    if !link_dir.exists() {
        fs::create_dir_all(link_dir).unwrap();
    }
    println!(
        "creating link {} to {}",
        link.display(),
        actual_path.display()
    );
    symlink(actual_path, link)?;
    Ok(())
}

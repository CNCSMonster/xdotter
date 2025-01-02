use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    /// dependencies,子成员的路径,每个子成员内部应该有配置
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<HashMap<String, String>>,
    /// 子成员路径',如果子成员路径存在,则在子成员路径中创建配置文件,左边为子成员路径,右边为目标链接创建路径
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<HashMap<String, String>>,
}

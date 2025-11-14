use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{RwLock, Arc};

/// 配置值类型
#[derive(Debug, Clone)]
pub enum ConfigValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Array(Vec<ConfigValue>),
    Object(HashMap<String, ConfigValue>),
}

impl ConfigValue {
    /// 转换为字符串
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ConfigValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// 转换为整数
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            ConfigValue::Int(i) => Some(*i),
            ConfigValue::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    /// 转换为浮点数
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ConfigValue::Float(f) => Some(*f),
            ConfigValue::Int(i) => Some(*i as f64),
            ConfigValue::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    /// 转换为布尔值
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ConfigValue::Bool(b) => Some(*b),
            ConfigValue::String(s) => match s.to_lowercase().as_str() {
                "true" | "yes" | "1" => Some(true),
                "false" | "no" | "0" => Some(false),
                _ => None,
            },
            _ => None,
        }
    }
}

/// 配置源 trait
pub trait PropertySource: Send + Sync {
    /// 获取配置源名称
    fn name(&self) -> &str;

    /// 获取配置值
    fn get(&self, key: &str) -> Option<ConfigValue>;

    /// 获取所有配置键
    fn keys(&self) -> Vec<String>;

    /// 配置源优先级（数字越大优先级越高）
    fn priority(&self) -> i32 {
        0
    }
}

/// Environment - 配置管理器
///
/// 类似 Spring Boot 的 Environment，提供统一的配置访问接口
pub struct Environment {
    /// 配置源列表（按优先级排序）
    sources: RwLock<Vec<Box<dyn PropertySource>>>,

    /// 当前激活的 profile
    active_profiles: RwLock<Vec<String>>,
}

impl std::fmt::Debug for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Environment")
            .field("active_profiles", &self.active_profiles)
            .field("sources_count", &self.sources.read().ok().map(|s| s.len()))
            .finish()
    }
}

impl Environment {
    /// 创建新的环境
    pub fn new() -> Self {
        Self {
            sources: RwLock::new(Vec::new()),
            active_profiles: RwLock::new(Vec::new()),
        }
    }

    /// 添加配置源
    pub fn add_property_source(&self, source: Box<dyn PropertySource>) {
        let mut sources = self.sources.write().unwrap();
        sources.push(source);
        // 按优先级降序排序
        sources.sort_by(|a, b| b.priority().cmp(&a.priority()));
    }

    /// 获取配置值
    pub fn get(&self, key: &str) -> Option<ConfigValue> {
        let sources = self.sources.read().unwrap();
        for source in sources.iter() {
            if let Some(value) = source.get(key) {
                tracing::debug!("Config '{}' found in source '{}'", key, source.name());
                return Some(value);
            }
        }
        tracing::debug!("Config '{}' not found in any source", key);
        None
    }

    /// 获取字符串配置
    pub fn get_string(&self, key: &str) -> Option<String> {
        self.get(key)
            .and_then(|v| v.as_str().map(String::from))
    }

    /// 获取字符串配置（带默认值）
    pub fn get_string_or(&self, key: &str, default: &str) -> String {
        self.get_string(key).unwrap_or_else(|| default.to_string())
    }

    /// 获取整数配置
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(|v| v.as_i64())
    }

    /// 获取整数配置（带默认值）
    pub fn get_i64_or(&self, key: &str, default: i64) -> i64 {
        self.get_i64(key).unwrap_or(default)
    }

    /// 获取浮点数配置
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(|v| v.as_f64())
    }

    /// 获取浮点数配置（带默认值）
    pub fn get_f64_or(&self, key: &str, default: f64) -> f64 {
        self.get_f64(key).unwrap_or(default)
    }

    /// 获取布尔值配置
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(|v| v.as_bool())
    }

    /// 获取布尔值配置（带默认值）
    pub fn get_bool_or(&self, key: &str, default: bool) -> bool {
        self.get_bool(key).unwrap_or(default)
    }

    /// 获取字符串数组配置
    /// 支持两种格式:
    /// 1. TOML数组: key = ["a", "b", "c"]
    /// 2. 逗号分隔字符串: key = "a, b, c"
    pub fn get_string_array(&self, key: &str) -> Option<Vec<String>> {
        match self.get(key)? {
            ConfigValue::Array(arr) => {
                Some(arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            }
            ConfigValue::String(s) => {
                Some(s.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect())
            }
            _ => None,
        }
    }

    /// 设置激活的 profile
    pub fn set_active_profiles(&self, profiles: Vec<String>) {
        let mut active = self.active_profiles.write().unwrap();
        *active = profiles;
    }

    /// 获取激活的 profile
    pub fn get_active_profiles(&self) -> Vec<String> {
        self.active_profiles.read().unwrap().clone()
    }

    /// 检查是否包含指定的 profile
    pub fn accepts_profiles(&self, profile: &str) -> bool {
        self.active_profiles.read().unwrap().contains(&profile.to_string())
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

/// Environment 实现 CoreComponent
impl crate::container::CoreComponent for Environment {
    fn core_bean_name() -> &'static str {
        crate::constants::ENVIRONMENT_BEAN_NAME
    }

    fn get_from_context(context: &Arc<crate::container::ApplicationContext>) -> Arc<Self> {
        Arc::clone(context.environment())
    }
}

// ========== Property Sources ==========

/// 环境变量配置源
pub struct EnvironmentPropertySource {
    prefix: String,
    priority: i32,
}

impl EnvironmentPropertySource {
    /// 创建环境变量配置源
    ///
    /// # 参数
    /// * `prefix` - 环境变量前缀，例如 "APP_"
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            priority: 100, // 环境变量优先级较高
        }
    }

    /// 将环境变量名转换为配置键
    /// 例如: APP_DATABASE_URL -> database.url
    fn env_to_key(&self, env_key: &str) -> String {
        if let Some(stripped) = env_key.strip_prefix(&self.prefix) {
            stripped.to_lowercase().replace('_', ".")
        } else {
            env_key.to_lowercase().replace('_', ".")
        }
    }

    /// 将配置键转换为环境变量名
    /// 例如: database.url -> APP_DATABASE_URL
    fn key_to_env(&self, key: &str) -> String {
        format!("{}{}", self.prefix, key.replace('.', "_").to_uppercase())
    }
}

impl PropertySource for EnvironmentPropertySource {
    fn name(&self) -> &str {
        "environment"
    }

    fn get(&self, key: &str) -> Option<ConfigValue> {
        let env_key = self.key_to_env(key);
        std::env::var(&env_key)
            .ok()
            .map(|v| ConfigValue::String(v))
    }

    fn keys(&self) -> Vec<String> {
        std::env::vars()
            .filter(|(k, _)| k.starts_with(&self.prefix))
            .map(|(k, _)| self.env_to_key(&k))
            .collect()
    }

    fn priority(&self) -> i32 {
        self.priority
    }
}

/// TOML 文件配置源
pub struct TomlPropertySource {
    name: String,
    properties: HashMap<String, ConfigValue>,
    priority: i32,
}

impl TomlPropertySource {
    /// 从文件加载 TOML 配置
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file {:?}: {}", path, e))?;

        Self::from_str(&content, path.to_string_lossy().to_string())
    }

    /// 从字符串解析 TOML 配置
    pub fn from_str(content: &str, name: String) -> Result<Self, String> {
        let value: toml::Value = toml::from_str(content)
            .map_err(|e| format!("Failed to parse TOML: {}", e))?;

        let mut properties = HashMap::new();
        Self::flatten_toml(&value, String::new(), &mut properties);

        Ok(Self {
            name,
            properties,
            priority: 0, // 文件配置优先级最低
        })
    }

    /// 展平 TOML 结构
    /// 例如: { database: { url: "xxx" } } -> { "database.url": "xxx" }
    fn flatten_toml(value: &toml::Value, prefix: String, result: &mut HashMap<String, ConfigValue>) {
        match value {
            toml::Value::String(s) => {
                result.insert(prefix, ConfigValue::String(s.clone()));
            }
            toml::Value::Integer(i) => {
                result.insert(prefix, ConfigValue::Int(*i));
            }
            toml::Value::Float(f) => {
                result.insert(prefix, ConfigValue::Float(*f));
            }
            toml::Value::Boolean(b) => {
                result.insert(prefix, ConfigValue::Bool(*b));
            }
            toml::Value::Array(arr) => {
                let values: Vec<ConfigValue> = arr
                    .iter()
                    .map(|v| Self::toml_value_to_config(v))
                    .collect();
                result.insert(prefix, ConfigValue::Array(values));
            }
            toml::Value::Table(table) => {
                for (key, val) in table {
                    let new_prefix = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };
                    Self::flatten_toml(val, new_prefix, result);
                }
            }
            toml::Value::Datetime(dt) => {
                result.insert(prefix, ConfigValue::String(dt.to_string()));
            }
        }
    }

    /// 转换 TOML 值为 ConfigValue
    fn toml_value_to_config(value: &toml::Value) -> ConfigValue {
        match value {
            toml::Value::String(s) => ConfigValue::String(s.clone()),
            toml::Value::Integer(i) => ConfigValue::Int(*i),
            toml::Value::Float(f) => ConfigValue::Float(*f),
            toml::Value::Boolean(b) => ConfigValue::Bool(*b),
            toml::Value::Array(arr) => {
                ConfigValue::Array(arr.iter().map(Self::toml_value_to_config).collect())
            }
            toml::Value::Table(table) => {
                let mut map = HashMap::new();
                for (k, v) in table {
                    map.insert(k.clone(), Self::toml_value_to_config(v));
                }
                ConfigValue::Object(map)
            }
            toml::Value::Datetime(dt) => ConfigValue::String(dt.to_string()),
        }
    }

    /// 设置优先级
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

impl PropertySource for TomlPropertySource {
    fn name(&self) -> &str {
        &self.name
    }

    fn get(&self, key: &str) -> Option<ConfigValue> {
        self.properties.get(key).cloned()
    }

    fn keys(&self) -> Vec<String> {
        self.properties.keys().cloned().collect()
    }

    fn priority(&self) -> i32 {
        self.priority
    }
}

/// 内存配置源（用于测试或运行时配置）
pub struct MapPropertySource {
    name: String,
    properties: HashMap<String, ConfigValue>,
    priority: i32,
}

impl MapPropertySource {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            properties: HashMap::new(),
            priority: 50,
        }
    }

    pub fn with_property(mut self, key: impl Into<String>, value: ConfigValue) -> Self {
        self.properties.insert(key.into(), value);
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

impl PropertySource for MapPropertySource {
    fn name(&self) -> &str {
        &self.name
    }

    fn get(&self, key: &str) -> Option<ConfigValue> {
        self.properties.get(key).cloned()
    }

    fn keys(&self) -> Vec<String> {
        self.properties.keys().cloned().collect()
    }

    fn priority(&self) -> i32 {
        self.priority
    }
}
use chimera_core_macros::ConfigurationProperties;

// ==================== 配置 ====================

#[derive(ConfigurationProperties, Debug, Clone)]
#[prefix("app")]
pub struct AppConfig {
    pub name: String,
    pub version: String,
}

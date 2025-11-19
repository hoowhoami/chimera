/// 核心组件常量定义

/// ApplicationContext 相关常量
pub const APPLICATION_CONTEXT_BEAN_NAME: &str = "applicationContext";

/// Environment 相关常量
pub const ENVIRONMENT_BEAN_NAME: &str = "environment";

/// EventPublisher 相关常量
pub const EVENT_PUBLISHER_BEAN_NAME: &str = "eventPublisher";

// ==================== 框架配置常量 ====================

/// 环境变量前缀
pub const ENV_PREFIX: &str = "CHIMERA_";

/// 默认应用名称（当配置文件未指定时使用）
pub const DEFAULT_APP_NAME: &str = "application";

/// 配置键：应用名称
pub const CONFIG_APP_NAME: &str = "chimera.app.name";

/// 配置键：应用版本
pub const CONFIG_APP_VERSION: &str = "chimera.app.version";

/// 配置键：激活的profiles
pub const CONFIG_PROFILES_ACTIVE: &str = "chimera.profiles.active";

/// 配置键：事件系统是否异步
pub const CONFIG_EVENTS_ASYNC: &str = "chimera.events.async";

/// 环境变量：激活的profiles
pub const ENV_PROFILES_ACTIVE: &str = "CHIMERA_PROFILES_ACTIVE";

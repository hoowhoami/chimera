/// 核心组件常量定义
///
/// 这个模块定义了所有核心组件的名称和类型路径常量，
/// 确保在宏和容器中使用相同的标识符，避免硬编码和不一致的问题
///
/// 当添加新的核心组件时，只需要在这里添加相应的常量即可

/// ApplicationContext 相关常量
pub const APPLICATION_CONTEXT_BEAN_NAME: &str = "applicationContext";
pub const APPLICATION_CONTEXT_TYPE_NAMES: &[&str] = &[
    "ApplicationContext",
    "chimera_core :: ApplicationContext",
    "chimera_core :: container :: ApplicationContext",
];

/// Environment 相关常量
pub const ENVIRONMENT_BEAN_NAME: &str = "environment";
pub const ENVIRONMENT_TYPE_NAMES: &[&str] = &[
    "Environment",
    "chimera_core :: Environment",
    "chimera_core :: config :: Environment",
];

/// EventPublisher 相关常量
pub const EVENT_PUBLISHER_BEAN_NAME: &str = "eventPublisher";
pub const EVENT_PUBLISHER_TYPE_NAMES: &[&str] = &[
    "AsyncEventPublisher",
    "chimera_core :: AsyncEventPublisher",
    "chimera_core :: event :: AsyncEventPublisher",
];

/// 所有核心组件类型名称的集合
/// 当添加新的核心组件时，记得在这里也添加相应的类型名称
pub const ALL_CORE_COMPONENT_TYPE_NAMES: &[&str] = &[
    // ApplicationContext 类型名称
    "ApplicationContext",
    "chimera_core :: ApplicationContext",
    "chimera_core :: container :: ApplicationContext",
    // Environment 类型名称
    "Environment",
    "chimera_core :: Environment",
    "chimera_core :: config :: Environment",
    // AsyncEventPublisher 类型名称
    "AsyncEventPublisher",
    "chimera_core :: AsyncEventPublisher",
    "chimera_core :: event :: AsyncEventPublisher",
];

/// 检查给定的类型名称是否为核心组件
///
/// # Arguments
/// * `type_name` - 要检查的类型名称
///
/// # Returns
/// * `bool` - 如果是核心组件返回 true，否则返回 false
///
/// # Example
/// ```
/// use chimera_core::core_components::is_core_component_type_name;
///
/// assert!(is_core_component_type_name("ApplicationContext"));
/// assert!(is_core_component_type_name("chimera_core :: Environment"));
/// assert!(!is_core_component_type_name("UserService"));
/// ```
pub fn is_core_component_type_name(type_name: &str) -> bool {
    ALL_CORE_COMPONENT_TYPE_NAMES.contains(&type_name)
        || type_name.contains("ApplicationContext")
        || type_name.contains("Environment")
        || type_name.contains("AsyncEventPublisher")
}

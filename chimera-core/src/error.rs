use thiserror::Error;

/// 容器错误类型
#[derive(Error, Debug)]
pub enum ContainerError {
    /// Bean 未找到
    #[error("Bean not found: {0}")]
    BeanNotFound(String),

    /// Bean 已存在
    #[error("Bean already exists: {0}")]
    BeanAlreadyExists(String),

    /// Bean 创建失败
    #[error("Failed to create bean: {0}")]
    BeanCreationFailed(String),

    /// 循环依赖
    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),

    /// 依赖验证失败
    #[error("Dependency validation failed: {0}")]
    DependencyValidationFailed(String),

    /// 类型不匹配
    #[error("Type mismatch: expected {expected}, found {found}")]
    TypeMismatch {
        expected: String,
        found: String,
    },

    /// 其他错误
    #[error("Container error: {0}")]
    Other(#[from] anyhow::Error),
}

/// 容器操作结果类型
pub type ContainerResult<T> = std::result::Result<T, ContainerError>;

/// 应用错误类型
#[derive(Error, Debug)]
pub enum ApplicationError {
    /// 日志初始化失败
    #[error("Failed to initialize logger: {0}")]
    LoggingInitFailed(String),

    /// 配置加载失败
    #[error("Failed to load configuration: {0}")]
    ConfigLoadFailed(String),

    /// 容器错误
    #[error("Container error: {0}")]
    Container(#[from] ContainerError),

    /// 其他应用错误
    #[error("Application error: {0}")]
    Other(String),
}

/// 应用操作结果类型
pub type ApplicationResult<T> = std::result::Result<T, ApplicationError>;


//! 框架配置常量定义
//!
//! 定义所有框架使用的配置键名称

// ==================== Server 配置 ====================

/// 服务器监听地址
pub const SERVER_HOST: &str = "server.host";

/// 服务器监听端口
pub const SERVER_PORT: &str = "server.port";

/// 工作线程数
pub const SERVER_WORKERS: &str = "server.workers";

/// 请求超时时间（秒）
pub const SERVER_REQUEST_TIMEOUT: &str = "server.request-timeout";

/// 是否启用 CORS
pub const SERVER_ENABLE_CORS: &str = "server.enable-cors";

/// 是否启用请求日志
pub const SERVER_ENABLE_REQUEST_LOGGING: &str = "server.enable-request-logging";

/// 是否启用全局异常处理
pub const SERVER_ENABLE_GLOBAL_EXCEPTION_HANDLING: &str = "server.enable-global-exception-handling";

// ==================== Multipart 配置 ====================

/// Multipart 最大文件大小（字节）
pub const MULTIPART_MAX_FILE_SIZE: &str = "chimera.web.multipart.max-file-size";

/// Multipart 最大字段数量
pub const MULTIPART_MAX_FIELDS: &str = "chimera.web.multipart.max-fields";

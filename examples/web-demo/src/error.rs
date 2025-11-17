use thiserror::Error;

/// 业务错误类型 - 用户自定义
/// 
/// 框架不再提供 ApplicationError，业务错误由用户自己定义
#[derive(Error, Debug)]
pub enum BusinessError {
    #[error("User not found: {0}")]
    UserNotFound(String),
    
    #[error("User already exists: {0}")]
    UserAlreadyExists(String),
    
    #[error("Invalid credentials")]
    InvalidCredentials,
    
    #[error("Insufficient permissions: {0}")]
    InsufficientPermissions(String),
    
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),
    
    #[error("Database error: {0}")]
    DatabaseError(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
}


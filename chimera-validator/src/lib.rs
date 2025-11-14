//! Chimera Validator - 参数验证模块
//!
//! 提供类似 Spring Boot Validator 的参数验证功能

pub mod error;
pub mod validator;
pub mod plugin;

pub use error::*;
pub use validator::*;
pub use plugin::*;

// 重新导出宏
pub use chimera_validator_macros::{Validate, valid};

//! 错误信息结构
//!
//! 提供结构化的错误信息传递给切面

use std::error::Error;

/// 结构化的错误信息
///
/// 用于在 after_throwing 通知中传递更丰富的错误信息
#[derive(Debug, Clone)]
pub struct ErrorInfo {
    /// 错误消息
    pub message: String,

    /// 错误类型名称
    pub error_type: String,

    /// 错误源链（cause chain）
    pub source_chain: Vec<String>,
}

impl ErrorInfo {
    /// 从标准错误创建 ErrorInfo
    pub fn from_error<E: Error>(error: &E) -> Self {
        let message = error.to_string();
        let error_type = std::any::type_name::<E>().to_string();

        // 构建错误源链
        let mut source_chain = Vec::new();
        let mut current_source = error.source();
        while let Some(source) = current_source {
            source_chain.push(source.to_string());
            current_source = source.source();
        }

        Self {
            message,
            error_type,
            source_chain,
        }
    }

    /// 创建简单的 ErrorInfo（只包含消息）
    pub fn simple(message: String) -> Self {
        Self {
            message,
            error_type: "Unknown".to_string(),
            source_chain: Vec::new(),
        }
    }

    /// 获取完整的错误描述（包含源链）
    pub fn full_description(&self) -> String {
        if self.source_chain.is_empty() {
            self.message.clone()
        } else {
            format!(
                "{}\nCaused by:\n  {}",
                self.message,
                self.source_chain.join("\n  ")
            )
        }
    }
}

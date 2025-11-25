/// 统一的错误处理类型
///
/// 使用 anyhow::Result 作为框架的统一错误类型，符合 Rust 社区最佳实践。
/// 通过 .context() 方法添加错误上下文信息。
///
/// # 示例
///
/// ```rust,ignore
/// use anyhow::{Context, Result};
///
/// fn get_bean(&self, name: &str) -> Result<Arc<dyn Any>> {
///     self.beans.get(name)
///         .cloned()
///         .ok_or_else(|| anyhow::anyhow!("Bean not found"))
///         .context(format!("Failed to get bean '{}'", name))
/// }
/// ```
pub use anyhow::Result;


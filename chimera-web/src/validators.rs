//! # 自定义验证器模块
//!
//! 提供常用的自定义验证函数，可以配合 `validator` 库的 `#[validate(custom)]` 属性使用

use validator::ValidationError;


/// 验证字符串不为空且不仅包含空白字符
///
/// 规则：
/// - 字符串不能为空
/// - 去除前后空格后不能为空字符串
/// - 不能仅包含空白字符（空格、制表符、换行符等）
///
/// ## 使用方法
///
/// ```ignore
/// use chimera_web::validators::*;
///
/// #[derive(Validate)]
/// struct User {
///     // 对于 String 类型
///     #[validate(custom(function = "validate_not_blank"))]
///     username: String,
///
///     // 对于 Option<String> 类型，使用 required + custom 组合
///     #[validate(required, custom(function = "validate_not_blank"))]
///     email: Option<String>,
/// }
/// ```
pub fn validate_not_blank(value: &str) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        let mut error = ValidationError::new("not_blank");
        error.message = Some("field must not be blank".into());
        Err(error)
    } else {
        Ok(())
    }
}

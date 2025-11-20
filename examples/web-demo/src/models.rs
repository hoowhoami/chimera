use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use validator::Validate;
// 导入框架提供的验证器
use chimera_web::validators::*;

// ==================== 数据模型 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub name: Option<String>,
    pub email: Option<String>,
    pub page: Option<u32>,
    pub size: Option<u32>,
}

static REGEX_PHONE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^1[3-9]\d{9}$").unwrap());

/// 用户注册请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct RegisterUserRequest {
    /// 用户名：不能为空，长度2-20个字符
    #[validate(
        required(message = "用户名不能为空"),
        custom(function = "validate_not_blank"),
        length(min = 2, max = 20, message = "用户名长度必须在2-20个字符之间")
    )]
    pub username: Option<String>,

    /// 邮箱：不能为空，必须是有效的邮箱格式
    #[validate(
        required(message = "邮箱不能为空"),
        custom(function = "validate_not_blank", message = "邮箱不能为空"),
        email(message = "请输入有效的邮箱地址")
    )]
    pub email: Option<String>,

    /// 密码：不能为空，最少8个字符
    #[validate(
        required(message = "密码不能为空"),
        custom(function = "validate_not_blank", message = "密码不能为空"),
        length(min = 8, message = "密码长度至少为8个字符")
    )]
    pub password: Option<String>,

    /// 年龄：必须在18-120之间
    #[validate(
        required(message = "年龄不能为空"),
        range(min = 18, max = 120, message = "年龄必须在18-120岁之间")
    )]
    pub age: Option<u32>,

    /// 手机号：必须匹配中国手机号格式
    #[validate(
        required(message = "手机号不能为空"),
        custom(function = "validate_not_blank", message = "手机号不能为空"),
        regex(path = "*REGEX_PHONE", message = "请输入有效的手机号")
    )]
    pub phone: Option<String>,
}

/// 用户登陆请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UserLoginRequest {
    /// 用户名：不能为空
    #[validate(
        required(message = "用户名不能为空"),
        custom(function = "validate_not_blank", message = "用户名不能为空")
    )]
    pub username: Option<String>,

    /// 密码：不能为空
    #[validate(
        required(message = "密码不能为空"),
        custom(function = "validate_not_blank", message = "密码不能为空")
    )]
    pub password: Option<String>,
}

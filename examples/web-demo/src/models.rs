use serde::{Deserialize, Serialize};
use chimera_validator::Validate;

// ==================== 数据模型 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub name: Option<String>,
    pub email: Option<String>,
    pub page: Option<u32>,
    pub size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
    pub remember_me: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentForm {
    pub author: String,
    pub content: String,
    pub rating: Option<u32>,
}

// ==================== 带验证的请求模型 ====================

/// 用户注册请求 - 演示参数验证功能
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct RegisterUserRequest {
    /// 用户名：不能为空，长度2-20个字符
    #[validate(not_blank)]
    #[validate(length_min = 2)]
    #[validate(length_max = 20)]
    pub username: String,

    /// 邮箱：不能为空，必须是有效的邮箱格式
    #[validate(not_blank)]
    #[validate(email)]
    pub email: String,

    /// 密码：不能为空，最少8个字符
    #[validate(not_blank)]
    #[validate(length_min = 8)]
    pub password: String,

    /// 年龄：必须在18-120之间
    #[validate(min = 18)]
    #[validate(max = 120)]
    pub age: u32,

    /// 手机号：必须匹配中国手机号格式
    #[validate(pattern = r"^1[3-9]\d{9}$")]
    pub phone: String,
}

/// 商品创建请求 - 演示更多验证规则
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateProductRequest {
    /// 商品名称：不能为空
    #[validate(not_blank)]
    pub name: String,

    /// 商品描述：不能为空，最少10个字符
    #[validate(not_blank)]
    #[validate(length_min = 10)]
    pub description: String,

    /// 价格：必须大于0
    #[validate(min = 1)]
    pub price: u32,

    /// 库存：必须在0-10000之间
    #[validate(min = 0)]
    #[validate(max = 10000)]
    pub stock: u32,
}


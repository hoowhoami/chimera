use serde::{Deserialize, Serialize};
use validator::Validate;
use once_cell::sync::Lazy;
use regex::Regex;

// 定义正则表达式用于验证
static PHONE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^1[3-9]\d{9}$").unwrap()
});

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

/// 带验证的登录表单 - 演示 ValidatedFormData 提取器
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ValidatedLoginForm {
    #[serde(default)]
    #[validate(length(min = 3, max = 20, message = "用户名长度必须在3-20个字符之间"))]
    pub username: String,

    #[serde(default)]
    #[validate(length(min = 6, message = "密码长度至少为6个字符"))]
    pub password: String,

    pub remember_me: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentForm {
    pub author: String,
    pub content: String,
    pub rating: Option<u32>,
}

/// 带验证的评论表单 - 演示 ValidatedFormData 提取器
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ValidatedCommentForm {
    #[serde(default)]
    #[validate(length(min = 2, max = 50, message = "作者名称长度必须在2-50个字符之间"))]
    pub author: String,

    #[serde(default)]
    #[validate(length(min = 10, max = 500, message = "评论内容长度必须在10-500个字符之间"))]
    pub content: String,

    #[serde(default)]
    #[validate(range(min = 1, max = 5, message = "评分必须在1-5之间"))]
    pub rating: u32,
}

/// 带验证的搜索查询参数 - 演示 ValidatedRequestParam 提取器
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ValidatedSearchQuery {
    #[serde(default)]
    #[validate(length(min = 2, max = 50, message = "搜索关键词长度必须在2-50个字符之间"))]
    pub keyword: String,

    #[serde(default = "default_page")]
    #[validate(range(min = 1, max = 1000, message = "页码必须在1-1000之间"))]
    pub page: u32,

    #[serde(default = "default_size")]
    #[validate(range(min = 1, max = 100, message = "每页数量必须在1-100之间"))]
    pub size: u32,
}

fn default_page() -> u32 { 1 }
fn default_size() -> u32 { 10 }

// ==================== 带验证的请求模型 ====================

/// 用户注册请求 - 演示自定义消息的参数验证功能
///
/// 使用 `#[serde(default)]` 让字段缺失时使用默认值，然后由 validator 验证
/// 这样更符合 Spring Boot 的行为：先完成反序列化，再验证
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct RegisterUserRequest {
    /// 用户名：不能为空，长度2-20个字符
    #[serde(default)]
    #[validate(length(min = 2, max = 20, message = "用户名长度必须在2-20个字符之间"))]
    pub username: String,

    /// 邮箱：不能为空，必须是有效的邮箱格式
    #[serde(default)]
    #[validate(email(message = "请输入有效的邮箱地址"))]
    pub email: String,

    /// 密码：不能为空，最少8个字符
    #[serde(default)]
    #[validate(length(min = 8, message = "密码长度至少为8个字符"))]
    pub password: String,

    /// 年龄：必须在18-120之间
    #[serde(default)]
    #[validate(range(min = 18, max = 120, message = "年龄必须在18-120岁之间"))]
    pub age: u32,

    /// 手机号：必须匹配中国手机号格式
    #[serde(default)]
    #[validate(regex(path = "*PHONE_REGEX", message = "请输入有效的手机号"))]
    pub phone: String,
}

/// 商品创建请求 - 演示更多自定义验证规则
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateProductRequest {
    /// 商品名称：不能为空
    #[serde(default)]
    #[validate(length(min = 1, message = "商品名称不能为空"))]
    pub name: String,

    /// 商品描述：不能为空，最少10个字符
    #[serde(default)]
    #[validate(length(min = 10, message = "商品描述至少需要10个字符"))]
    pub description: String,

    /// 价格：必须大于0
    #[serde(default)]
    #[validate(range(min = 1, message = "商品价格必须大于0"))]
    pub price: u32,

    /// 库存：必须在0-10000之间
    #[serde(default)]
    #[validate(range(min = 0, max = 10000, message = "库存数量必须在0-10000之间"))]
    pub stock: u32,
}

/// 用户更新请求 - 演示所有验证器对 Option 类型的支持
/// 使用 Option<T> 适合部分更新场景（PATCH），字段不存在时不更新
///
/// 所有验证器都支持 Option 类型：
/// - None 值：跳过验证
/// - Some(值)：正常验证该值
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateUserWithPhone {
    /// 用户名：可选，如果提供则验证长度
    #[validate(length(min = 2, max = 20, message = "用户名长度必须在2-20个字符之间"))]
    pub username: Option<String>,

    /// 邮箱：可选，如果提供则验证格式
    #[validate(email(message = "请输入有效的邮箱地址"))]
    pub email: Option<String>,

    /// 手机号：可选，如果提供则验证格式（中国手机号）
    /// 演示 Option<String> 使用 regex 验证
    #[validate(regex(path = "*PHONE_REGEX", message = "请输入有效的手机号"))]
    pub phone: Option<String>,

    /// 年龄：可选，如果提供则验证范围
    /// 演示 Option<u32> 使用 range 验证
    #[validate(range(min = 18, max = 120, message = "年龄必须在18-120岁之间"))]
    pub age: Option<u32>,

    /// 个人简介：可选，如果提供则不能为空
    /// 演示 Option<String> 使用 length 验证
    #[validate(length(min = 1, message = "个人简介不能为空字符串"))]
    pub bio: Option<String>,

    /// 昵称：可选，如果提供则不能为空白
    /// 演示 Option<String> 使用 length 验证
    #[validate(length(min = 1, message = "昵称不能为空白字符"))]
    pub nickname: Option<String>,
}

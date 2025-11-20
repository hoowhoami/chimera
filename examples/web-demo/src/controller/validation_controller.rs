use chimera_core::prelude::*;
use chimera_core_macros::Component;
use chimera_web_macros::{Controller, controller, post_mapping};
use chimera_web::prelude::*;
use chimera_web::extractors::ValidatedRequestBody;
use crate::models::{CreateArticleRequest, UserSignupRequest};

/// 验证器演示 Controller
///
/// 展示如何使用框架内置验证器和自定义验证器
#[derive(Controller, Component, Clone)]
#[route("/validation")]
pub struct ValidationController;

#[controller]
impl ValidationController {
    /// 演示框架内置验证器 - 创建文章
    #[post_mapping("/article")]
    pub async fn create_article(
        &self,
        ValidatedRequestBody(article): ValidatedRequestBody<CreateArticleRequest>,
    ) -> impl IntoResponse {
        ResponseEntity::ok(serde_json::json!({
            "success": true,
            "message": "文章创建成功",
            "data": {
                "title": article.title,
                "content": article.content,
                "author": article.author,
                "tags": article.tags,
            }
        }))
    }

    /// 演示用户自定义验证器 - 用户注册
    #[post_mapping("/signup")]
    pub async fn user_signup(
        &self,
        ValidatedRequestBody(user): ValidatedRequestBody<UserSignupRequest>,
    ) -> impl IntoResponse {
        ResponseEntity::ok(serde_json::json!({
            "success": true,
            "message": "用户注册成功",
            "data": {
                "username": user.username,
                "email": user.email,
                "nickname": user.nickname,
            }
        }))
    }
}

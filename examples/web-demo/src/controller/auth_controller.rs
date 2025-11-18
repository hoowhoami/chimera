use chimera_core::prelude::*;
use chimera_core_macros::Component;
use chimera_web_macros::{Controller, controller, get_mapping, post_mapping};
use chimera_web::prelude::*;
use chimera_web::extractors::{RequestBody, FormData};
use serde_json::json;

use crate::models::LoginForm;

/// 认证授权 Controller
///
/// 提供用户认证、授权相关端点
#[derive(Controller, Component, Clone)]
#[route("/auth")]
pub struct AuthController;

#[controller]
impl AuthController {

    /// 用户登录（表单提交）
    #[post_mapping("/login")]
    async fn login(&self, FormData(form): FormData<LoginForm>) -> impl IntoResponse {
        ResponseEntity::ok(json!({
            "message": "登录成功",
            "username": form.username,
            "remember_me": form.remember_me.unwrap_or(false),
            "note": "使用 FormData 提取器处理表单提交"
        }))
    }


    /// 认证登录（JSON格式）
    #[post_mapping("/auth/login")]
    async fn auth_login(&self, RequestBody(credentials): RequestBody<serde_json::Value>) -> impl IntoResponse {
        let username = credentials.get("username")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let token = if username == "admin" {
            "admin-token"
        } else {
            "valid-token"
        };

        ResponseEntity::ok(json!({
            "message": "登录成功",
            "token": token,
            "username": username,
            "note": "使用这个token在Authorization头中进行后续请求: 'Bearer <token>'"
        }))
    }


    /// 受保护的端点（需要认证）
    #[get_mapping("/auth/protected")]
    async fn protected_endpoint(&self) -> impl IntoResponse {
        ResponseEntity::ok(json!({
            "message": "恭喜！您已成功通过身份验证",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "note": "这个端点需要有效的JWT token才能访问"
        }))
    }


    /// 管理员面板（需要管理员权限）
    #[get_mapping("/admin/panel")]
    async fn admin_panel(&self) -> impl IntoResponse {
        ResponseEntity::ok(json!({
            "message": "欢迎进入管理员面板",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "note": "这个端点需要ADMIN角色才能访问"
        }))
    }


    /// 公开端点（无需认证）
    #[get_mapping("/public/info")]
    async fn public_info(&self) -> impl IntoResponse {
        ResponseEntity::ok(json!({
            "message": "这是一个公开端点，无需认证",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "note": "此端点在AuthInterceptor的排除列表中"
        }))
    }
}

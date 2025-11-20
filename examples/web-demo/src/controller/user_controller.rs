use crate::models::{RegisterUserRequest, SearchQuery, UserLoginRequest};
use crate::service::UserService;
use chimera_core::prelude::*;
use chimera_core_macros::Component;
use chimera_web::extractors::ValidatedRequestBody;
use chimera_web::prelude::*;
use chimera_web_macros::{controller, post_mapping};
use std::sync::Arc;

/// 用户 Controller
///
/// 展示如何使用框架内置验证器和自定义验证器
#[controller("/user")]
#[derive(Component, Clone)]
pub struct UserController {
    #[autowired]
    user_service: Arc<UserService>,
}

#[controller]
impl UserController {
    #[post_mapping("/search")]
    async fn search(
        &self,
        RequestParam(query): RequestParam<SearchQuery>,
    ) -> impl IntoResponse {
        let users = self.user_service.search_users(query);
        ResponseEntity::ok(serde_json::json!({
            "success": true,
            "message": "search success",
            "data": users
        }))
    }

    #[post_mapping("/register")]
    async fn register(
        &self,
        ValidatedRequestBody(user): ValidatedRequestBody<RegisterUserRequest>,
    ) -> impl IntoResponse {
        ResponseEntity::ok(serde_json::json!({
            "success": true,
            "message": "register success",
            "data": user
        }))
    }

    #[post_mapping("/login")]
    async fn login(
        &self,
        ValidatedFormData(user): ValidatedFormData<UserLoginRequest>,
    ) -> impl IntoResponse {
        ResponseEntity::ok(serde_json::json!({
            "success": true,
            "message": "login success",
            "data": user
        }))
    }
}

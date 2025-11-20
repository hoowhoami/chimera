use crate::models::{RegisterUserRequest, SearchQuery, UserLoginRequest};
use crate::service::UserService;
use chimera_core::prelude::*;
use chimera_core_macros::{component, Component};
use chimera_web::extractors::{RequestParam, ValidatedFormData, ValidatedRequestBody};
use chimera_web::prelude::*;
use chimera_web_macros::{controller, get_mapping, post_mapping, request_mapping};
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

#[component]
#[controller]
impl UserController {
    #[request_mapping("/search")]
    async fn search(&self, RequestParam(query): RequestParam<SearchQuery>) -> impl IntoResponse {
        let users = self.user_service.search_users(query);
        ResponseEntity::ok(serde_json::json!({
            "success": true,
            "message": "search success",
            "data": users
        }))
    }

    #[get_mapping("/:id")]
    async fn get_user(&self, PathVariable(id): PathVariable<u32>) -> impl IntoResponse {
        let user = self.user_service.get_user_by_id(id);
        ResponseEntity::ok(serde_json::json!({
            "success": true,
            "message": "search success",
            "data": user
        }))
    }

    /// 用户注册
    #[post_mapping("/register")]
    async fn user_register(
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
        ValidatedFormData(login_form): ValidatedFormData<UserLoginRequest>,
    ) -> impl IntoResponse {
        ResponseEntity::ok(serde_json::json!({
            "success": true,
            "message": "login success",
            "data": {
                "username": login_form.username
            }
        }))
    }
}

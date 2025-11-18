use chimera_core::prelude::*;
use chimera_core_macros::Component;
use chimera_web_macros::{Controller, controller, get_mapping, post_mapping, put_mapping};
use chimera_web::prelude::*;
use chimera_web::extractors::{PathVariable, RequestBody, RequestParam, ValidatedRequestBody, Cookies};
use serde_json::json;
use std::sync::Arc;

use crate::service::UserService;
use crate::models::{CreateUserRequest, UpdateUserRequest, SearchQuery, RegisterUserRequest};

/// 用户管理 Controller
///
/// 提供用户 CRUD 操作端点
#[derive(Controller, Component, Clone)]
#[route("/api")]
pub struct UserController {
    #[autowired]
    user_service: Arc<UserService>,
}

#[controller]
impl UserController {
    /// GET /api/users
    /// 获取所有用户列表
    #[get_mapping("/users")]
    async fn list_users(&self) -> impl IntoResponse {
        let users = self.user_service.list_users();
        ResponseEntity::ok(users)
    }

    /// GET /api/users/:id
    /// 根据ID获取单个用户
    #[get_mapping("/users/:id")]
    async fn get_user(&self, PathVariable(id): PathVariable<u32>) -> impl IntoResponse {
        match self.user_service.get_user_by_id(id) {
            Some(user) => ResponseEntity::ok(user).into_response(),
            None => ResponseEntity::not_found(json!({
                "error": "User not found",
                "id": id
            })).into_response()
        }
    }

    /// POST /api/users/create
    /// 创建新用户
    #[post_mapping("/users/create")]
    async fn create_user(&self, RequestBody(request): RequestBody<CreateUserRequest>) -> impl IntoResponse {
        let user = self.user_service.create_user(request);
        ResponseEntity::created(user)
    }

    /// PUT /api/users/:id
    /// 更新用户信息
    #[put_mapping("/users/:id")]
    async fn update_user(
        &self,
        PathVariable(id): PathVariable<u32>,
        RequestBody(request): RequestBody<UpdateUserRequest>,
    ) -> impl IntoResponse {
        match self.user_service.update_user(id, request) {
            Some(user) => ResponseEntity::ok(user).into_response(),
            None => ResponseEntity::not_found(json!({
                "error": "User not found"
            })).into_response()
        }
    }

    /// GET /api/users/search?name=Alice&page=1
    /// 搜索用户
    #[get_mapping("/users/search")]
    async fn search_users(&self, RequestParam(query): RequestParam<SearchQuery>) -> impl IntoResponse {
        let users = self.user_service.search_users(query);
        ResponseEntity::ok(users)
    }

    /// POST /api/register
    /// 用户注册（带参数验证）
    #[post_mapping("/register")]
    async fn user_register(&self, ValidatedRequestBody(request): ValidatedRequestBody<RegisterUserRequest>) -> impl IntoResponse {
        ResponseEntity::ok(json!({
            "message": "用户注册成功",
            "username": request.username,
            "email": request.email,
            "age": request.age,
            "phone": request.phone,
            "note": "所有参数验证已通过"
        }))
    }

    /// GET /api/users/:id/preferences
    /// 获取用户偏好设置（演示 PathVariable + Cookies 组合）
    #[get_mapping("/users/:id/preferences")]
    async fn get_user_preferences(
        &self,
        PathVariable(user_id): PathVariable<u32>,
        Cookies(cookies): Cookies,
    ) -> impl IntoResponse {
        let theme = cookies.get("theme").cloned().unwrap_or_else(|| "light".to_string());
        let language = cookies.get("language").cloned().unwrap_or_else(|| "en-US".to_string());
        let timezone = cookies.get("timezone").cloned().unwrap_or_else(|| "UTC".to_string());

        ResponseEntity::ok(json!({
            "user_id": user_id,
            "preferences": {
                "theme": theme,
                "language": language,
                "timezone": timezone
            },
            "note": "组合使用 PathVariable 和 Cookies 提取器"
        }))
    }
}

use chimera_core::prelude::*;
use chimera_core_macros::Component;
use chimera_web_macros::{Controller, controller, get_mapping, post_mapping};
use chimera_web::prelude::*;
use chimera_web::extractors::{
    ValidatedRequestParam, ValidatedFormData, ValidatedRequestBody,
    RequestBody, PathVariable, RequestParam, FormData, RequestHeaders,
    Cookies, Session
};
use chimera_web::exception_handler::WebError;
use serde_json::json;

use crate::models::{
    CommentForm, ValidatedLoginForm, ValidatedCommentForm, ValidatedSearchQuery,
    CreateProductRequest
};
use crate::error::BusinessError;

/// 测试 Controller
///
/// 提供各种测试端点，包括验证测试、异常测试、限流测试等
#[derive(Controller, Component, Clone)]
#[route("/api")]
pub struct TestController;

#[controller]
impl TestController {
    // ========== 验证提取器测试 ==========

    /// POST /api/test/validated-form
    /// 测试 ValidatedFormData 提取器
    #[post_mapping("/test/validated-form")]
    async fn test_validated_form(&self, ValidatedFormData(form): ValidatedFormData<ValidatedLoginForm>) -> impl IntoResponse {
        ResponseEntity::ok(json!({
            "message": "表单验证通过",
            "username": form.username,
            "remember_me": form.remember_me.unwrap_or(false),
            "note": "使用 ValidatedFormData 提取器，自动验证表单数据"
        }))
    }

    /// POST /api/test/validated-comment
    /// 测试评论表单验证
    #[post_mapping("/test/validated-comment")]
    async fn test_validated_comment(&self, ValidatedFormData(comment): ValidatedFormData<ValidatedCommentForm>) -> impl IntoResponse {
        ResponseEntity::ok(json!({
            "message": "评论提交成功",
            "author": comment.author,
            "content": comment.content,
            "rating": comment.rating,
            "note": "使用 ValidatedFormData 提取器验证评论表单"
        }))
    }

    /// GET /api/test/validated-search
    /// 测试查询参数验证
    #[get_mapping("/test/validated-search")]
    async fn test_validated_search(&self, ValidatedRequestParam(query): ValidatedRequestParam<ValidatedSearchQuery>) -> impl IntoResponse {
        ResponseEntity::ok(json!({
            "message": "搜索参数验证通过",
            "keyword": query.keyword,
            "page": query.page,
            "size": query.size,
            "note": "使用 ValidatedRequestParam 提取器验证查询参数"
        }))
    }

    /// POST /api/products
    /// 测试商品创建（参数验证）
    #[post_mapping("/products")]
    async fn create_product(&self, ValidatedRequestBody(request): ValidatedRequestBody<CreateProductRequest>) -> impl IntoResponse {
        ResponseEntity::ok(json!({
            "message": "商品创建成功",
            "product": {
                "id": 1001,
                "name": request.name,
                "description": request.description,
                "price": request.price,
                "stock": request.stock
            },
            "note": "商品信息验证通过"
        }))
    }

    // ========== 异常处理器测试 ==========

    /// GET /api/test/business-error
    /// 测试业务异常处理
    #[get_mapping("/test/business-error")]
    async fn test_business_error(&self) -> impl IntoResponse {
        let error = BusinessError::ValidationError(
            "用户名必须长度在2-20个字符之间".to_string()
        );
        WebError::UserDefined(Box::new(error)).into_response()
    }

    /// GET /api/test/user-not-found
    /// 测试用户不存在错误
    #[get_mapping("/test/user-not-found")]
    async fn test_user_not_found(&self) -> impl IntoResponse {
        let error = BusinessError::UserNotFound("user_123".to_string());
        WebError::UserDefined(Box::new(error)).into_response()
    }

    /// GET /api/test/database-error
    /// 测试数据库异常
    #[get_mapping("/test/database-error")]
    async fn test_database_error(&self) -> impl IntoResponse {
        let error = BusinessError::DatabaseError(
            "Database connection timeout after 30 seconds".to_string()
        );
        WebError::UserDefined(Box::new(error)).into_response()
    }

    /// GET /api/test/generic-error
    /// 测试通用错误处理（panic）
    #[get_mapping("/test/generic-error")]
    async fn test_generic_error(&self) -> impl IntoResponse {
        panic!("模拟系统panic，用于测试全局异常处理")
    }

    /// GET /api/test/rate-limit
    /// 测试限流拦截器
    #[get_mapping("/test/rate-limit")]
    async fn test_rate_limit(&self) -> impl IntoResponse {
        ResponseEntity::ok(json!({
            "message": "限流测试成功",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "note": "连续快速请求此端点以测试限流功能"
        }))
    }

    // ========== 提取器演示 ==========

    /// GET /api/headers
    /// 演示 RequestHeaders 提取器
    #[get_mapping("/headers")]
    async fn get_headers(&self, RequestHeaders(headers): RequestHeaders) -> impl IntoResponse {
        let user_agent = headers.get("user-agent")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");

        let content_type = headers.get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("not specified");

        ResponseEntity::ok(json!({
            "message": "请求头信息",
            "user_agent": user_agent,
            "content_type": content_type,
            "total_headers": headers.len(),
            "note": "使用 RequestHeaders 提取器获取所有请求头"
        }))
    }

    /// GET /api/cookies
    /// 演示 Cookies 提取器
    #[get_mapping("/cookies")]
    async fn get_cookies(&self, Cookies(cookies): Cookies) -> impl IntoResponse {
        let session_id = cookies.get("session_id").cloned();
        let user_id = cookies.get("user_id").cloned();
        let theme = cookies.get("theme").cloned();

        ResponseEntity::ok(json!({
            "message": "Cookie 提取成功",
            "cookies": {
                "session_id": session_id,
                "user_id": user_id,
                "theme": theme,
                "total": cookies.len()
            },
            "note": "使用 Cookies 提取器获取所有 Cookie"
        }))
    }

    /// GET /api/session/user
    /// 演示 Session 提取器
    #[get_mapping("/session/user")]
    async fn get_session_user(&self, Session(session): Session<serde_json::Value>) -> impl IntoResponse {
        ResponseEntity::ok(json!({
            "message": "Session 提取成功",
            "session_data": session,
            "note": "使用 Session 提取器从 Cookie 中提取会话信息（JSON 格式）"
        }))
    }

    /// POST /api/users/:id/comments
    /// 演示 PathVariable + FormData 组合
    #[post_mapping("/users/:id/comments")]
    async fn add_comment(
        &self,
        PathVariable(user_id): PathVariable<u32>,
        FormData(comment): FormData<CommentForm>,
    ) -> impl IntoResponse {
        ResponseEntity::ok(json!({
            "message": "评论添加成功",
            "user_id": user_id,
            "comment": {
                "author": comment.author,
                "content": comment.content,
                "rating": comment.rating
            },
            "note": "组合使用 PathVariable 和 FormData 提取器"
        }))
    }

    /// POST /api/users/:id/actions
    /// 演示复杂组合：PathVariable + RequestParam + RequestBody
    #[post_mapping("/users/:id/actions")]
    async fn complex_action(
        &self,
        PathVariable(id): PathVariable<u32>,
        RequestParam(params): RequestParam<serde_json::Value>,
        RequestBody(body): RequestBody<serde_json::Value>,
    ) -> impl IntoResponse {
        ResponseEntity::ok(json!({
            "message": "复杂操作成功",
            "user_id": id,
            "query_params": params,
            "request_body": body,
            "note": "同时使用了 PathVariable, RequestParam, RequestBody 三种提取器"
        }))
    }

    /// GET /api/users/:id/metadata
    /// 演示 PathVariable + RequestHeaders 组合
    #[get_mapping("/users/:id/metadata")]
    async fn get_user_metadata(
        &self,
        PathVariable(id): PathVariable<u32>,
        RequestHeaders(headers): RequestHeaders,
    ) -> impl IntoResponse {
        let authorization = headers.get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("not provided");

        ResponseEntity::ok(json!({
            "user_id": id,
            "authorization": authorization,
            "note": "组合使用 PathVariable 和 RequestHeaders 提取器"
        }))
    }
}

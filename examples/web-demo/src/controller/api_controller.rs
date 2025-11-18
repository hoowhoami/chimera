use chimera_core::prelude::*;
use chimera_core_macros::Component;
use chimera_web_macros::{Controller, controller, get_mapping, post_mapping, put_mapping, request_mapping};
use chimera_web::prelude::*;
use chimera_web::extractors::{PathVariable, RequestBody, RequestParam, FormData, RequestHeaders, ValidatedRequestBody, ValidatedFormData, Cookies, Session};
use chimera_web::exception_handler::WebError;
use serde_json::json;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::service::UserService;
use crate::models::{CreateUserRequest, UpdateUserRequest, SearchQuery, LoginForm, CommentForm,
                  RegisterUserRequest, CreateProductRequest, ValidatedLoginForm,
                  ValidatedCommentForm, ValidatedSearchQuery};
use crate::error::BusinessError;

#[derive(Controller, Component, Clone)]
#[route("/api")]
pub struct ApiController {
    #[autowired]
    user_service: Arc<UserService>,

    #[autowired]
    config: Arc<AppConfig>,
}

#[controller]
impl ApiController {
    // ========== 无参数方法 ==========

    /// GET /api/info
    #[get_mapping("/info")]
    async fn get_info(&self) -> impl IntoResponse {
        ResponseEntity::ok(json!({
            "app": self.config.name,
            "version": self.config.version,
            "status": "running"
        }))
    }

    /// GET /api/users
    #[get_mapping("/users")]
    async fn list_users(&self) -> impl IntoResponse {
        let users = self.user_service.list_users();
        ResponseEntity::ok(users)
    }

    /// GET/POST/PUT/DELETE /api/health
    #[request_mapping("/health")]
    async fn health_check(&self) -> impl IntoResponse {
        ResponseEntity::ok(json!({
            "status": "healthy",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }

    // ========== 使用 PathVariable 提取器 ==========

    /// GET /api/users/:id
    /// 使用 PathVariable 提取路径参数
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

    // ========== 使用 RequestBody 提取器 ==========

    /// POST /api/users/create
    /// 使用 RequestBody 提取 JSON 请求体
    #[post_mapping("/users/create")]
    async fn create_user(&self, RequestBody(request): RequestBody<CreateUserRequest>) -> impl IntoResponse {
        let user = self.user_service.create_user(request);
        ResponseEntity::created(user)
    }

    // ========== 组合 PathVariable + RequestBody ==========

    /// PUT /api/users/:id
    /// 同时使用路径参数和请求体
    #[put_mapping("/users/:id")]
    async fn update_user(
        &self,
        PathVariable(id): PathVariable<u32>,                    // 路径参数
        RequestBody(request): RequestBody<UpdateUserRequest>,   // 请求体
    ) -> impl IntoResponse {
        match self.user_service.update_user(id, request) {
            Some(user) => ResponseEntity::ok(user).into_response(),
            None => ResponseEntity::not_found(json!({
                "error": "User not found"
            })).into_response()
        }
    }

    // ========== 使用 RequestParam 提取器 ==========

    /// GET /api/users/search?name=Alice&page=1
    /// 使用 RequestParam 提取 query 参数
    #[get_mapping("/users/search")]
    async fn search_users(&self, RequestParam(query): RequestParam<SearchQuery>) -> impl IntoResponse {
        let users = self.user_service.search_users(query);
        ResponseEntity::ok(users)
    }

    // ========== 使用 Controller 字段注入的服务 ==========

    /// GET /api/demo/service
    /// 演示使用 Controller 字段注入的服务
    #[get_mapping("/demo/service")]
    async fn demo_service(&self) -> impl IntoResponse {
        // 直接使用 controller 字段中注入的 user_service
        let users = self.user_service.list_users();
        ResponseEntity::ok(json!({
            "message": "演示 Controller 字段注入",
            "users": users,
            "note": "user_service 是通过 Controller 字段的 #[autowired] 注入的"
        }))
    }

    // ========== 复杂组合：PathVariable + RequestParam + RequestBody ==========

    /// POST /api/users/:id/actions?notify=true&async=false
    /// Body: {"name": "New Name"}
    ///
    /// 同时使用三种提取器
    #[post_mapping("/users/:id/actions")]
    async fn complex_action(
        &self,
        PathVariable(id): PathVariable<u32>,                    // 路径参数
        RequestParam(params): RequestParam<serde_json::Value>,  // Query 参数
        RequestBody(body): RequestBody<serde_json::Value>,      // 请求体
    ) -> impl IntoResponse {
        ResponseEntity::ok(json!({
            "message": "复杂操作成功",
            "user_id": id,
            "query_params": params,
            "request_body": body,
            "note": "同时使用了 PathVariable, RequestParam, RequestBody 三种提取器"
        }))
    }

    // ========== 使用 FormData 提取器 ==========

    /// POST /api/login
    /// Content-Type: application/x-www-form-urlencoded
    /// Body: username=alice&password=secret&remember_me=true
    ///
    /// 演示使用 FormData 提取表单数据
    #[post_mapping("/login")]
    async fn login(&self, FormData(form): FormData<LoginForm>) -> impl IntoResponse {
        ResponseEntity::ok(json!({
            "message": "登录成功",
            "username": form.username,
            "remember_me": form.remember_me.unwrap_or(false),
            "note": "使用 FormData 提取器处理表单提交"
        }))
    }

    /// POST /api/users/:id/comments
    /// Content-Type: application/x-www-form-urlencoded
    /// Body: author=John&content=Great!&rating=5
    ///
    /// 组合使用 PathVariable 和 FormData
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

    // ========== 使用 RequestHeaders 提取器 ==========

    /// GET /api/headers
    /// 使用 RequestHeaders 提取所有请求头
    #[get_mapping("/headers")]
    async fn get_headers(&self, RequestHeaders(headers): RequestHeaders) -> impl IntoResponse {
        let user_agent = headers.get("user-agent")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");

        let content_type = headers.get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("not specified");

        let accept = headers.get("accept")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("*/*");

        ResponseEntity::ok(json!({
            "message": "请求头信息",
            "user_agent": user_agent,
            "content_type": content_type,
            "accept": accept,
            "total_headers": headers.len(),
            "note": "使用 RequestHeaders 提取器获取所有请求头"
        }))
    }

    /// GET /api/users/:id/metadata
    /// 组合 PathVariable 和 RequestHeaders
    #[get_mapping("/users/:id/metadata")]
    async fn get_user_metadata(
        &self,
        PathVariable(id): PathVariable<u32>,
        RequestHeaders(headers): RequestHeaders,
    ) -> impl IntoResponse {
        let authorization = headers.get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("not provided");

        let user_agent = headers.get("user-agent")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");

        ResponseEntity::ok(json!({
            "user_id": id,
            "authorization": authorization,
            "user_agent": user_agent,
            "note": "组合使用 PathVariable 和 RequestHeaders 提取器"
        }))
    }

    // ========== 测试验证提取器的端点 ==========

    /// POST /api/test/validated-form
    /// 测试 ValidatedFormData 提取器 - 表单验证
    ///
    /// 使用 application/x-www-form-urlencoded 格式提交表单
    ///
    /// 示例（缺失字段测试）:
    /// curl -X POST http://localhost:3000/api/test/validated-form \
    ///   -H "Content-Type: application/x-www-form-urlencoded" \
    ///   -d "username="
    ///
    /// 示例（正常提交）:
    /// curl -X POST http://localhost:3000/api/test/validated-form \
    ///   -H "Content-Type: application/x-www-form-urlencoded" \
    ///   -d "username=testuser&password=password123&remember_me=true"
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
    /// 测试 ValidatedFormData 提取器 - 评论表单验证
    ///
    /// 示例（评分超出范围）:
    /// curl -X POST http://localhost:3000/api/test/validated-comment \
    ///   -H "Content-Type: application/x-www-form-urlencoded" \
    ///   -d "author=张三&content=这是一条评论内容&rating=10"
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
    /// 测试 ValidatedRequestParam 提取器 - 查询参数验证
    ///
    /// 示例（缺失关键词）:
    /// curl "http://localhost:3000/api/test/validated-search?page=1&size=20"
    ///
    /// 示例（页码超出范围）:
    /// curl "http://localhost:3000/api/test/validated-search?keyword=test&page=9999&size=20"
    ///
    /// 示例（正常请求）:
    /// curl "http://localhost:3000/api/test/validated-search?keyword=rust&page=2&size=50"
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

    // ========== 测试异常处理器的端点 ==========

    /// GET /api/test/business-error
    /// 测试业务异常处理器
    #[get_mapping("/test/business-error")]
    async fn test_business_error(&self) -> impl IntoResponse {
        // 触发业务异常，会被 BusinessExceptionHandler 处理
        let error = BusinessError::ValidationError(
            "用户名必须长度在2-20个字符之间".to_string()
        );
        // 将业务错误包装到 WebError 中
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
    /// 测试数据库异常处理器
    #[get_mapping("/test/database-error")]
    async fn test_database_error(&self) -> impl IntoResponse {
        // 模拟数据库连接错误
        let error = BusinessError::DatabaseError(
            "Database connection timeout after 30 seconds".to_string()
        );
        WebError::UserDefined(Box::new(error)).into_response()
    }

    /// GET /api/test/generic-error
    /// 测试通用错误处理
    #[get_mapping("/test/generic-error")]
    async fn test_generic_error(&self) -> impl IntoResponse {
        // 模拟一个会导致panic的情况
        panic!("模拟系统panic，用于测试全局异常处理")
    }

    // ========== 测试拦截器的端点 ==========

    /// GET /api/auth/protected
    /// 需要认证的端点，会被 AuthInterceptor 拦截
    #[get_mapping("/auth/protected")]
    async fn protected_endpoint(&self) -> impl IntoResponse {
        ResponseEntity::ok(json!({
            "message": "恭喜！您已成功通过身份验证",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "note": "这个端点需要有效的JWT token才能访问"
        }))
    }

    /// GET /api/admin/panel
    /// 需要管理员权限的端点
    #[get_mapping("/admin/panel")]
    async fn admin_panel(&self) -> impl IntoResponse {
        ResponseEntity::ok(json!({
            "message": "欢迎进入管理员面板",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "note": "这个端点需要ADMIN角色才能访问"
        }))
    }

    /// POST /api/auth/login
    /// 登录端点（不需要认证），返回测试用的token
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

    /// GET /api/public/info
    /// 公开端点，不需要认证
    #[get_mapping("/public/info")]
    async fn public_info(&self) -> impl IntoResponse {
        ResponseEntity::ok(json!({
            "message": "这是一个公开端点，无需认证",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "note": "此端点在AuthInterceptor的排除列表中"
        }))
    }

    /// GET /api/test/rate-limit
    /// 测试限流拦截器的端点
    #[get_mapping("/test/rate-limit")]
    async fn test_rate_limit(&self) -> impl IntoResponse {
        ResponseEntity::ok(json!({
            "message": "限流测试成功",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "note": "连续快速请求此端点以测试限流功能"
        }))
    }

    // ========== 参数验证示例端点 ==========

    /// POST /api/register
    /// 用户注册 - 演示自动验证
    ///
    /// 使用 ValidatedRequestBody 自动验证请求参数
    /// 如果验证失败，会自动返回 400 错误和详细的验证错误信息
    ///
    /// 示例请求体：
    /// ```json
    /// {
    ///   "username": "alice",
    ///   "email": "alice@example.com",
    ///   "password": "password123",
    ///   "age": 25,
    ///   "phone": "13800138000"
    /// }
    /// ```
    #[post_mapping("/register")]
    async fn user_register(&self, ValidatedRequestBody(request): ValidatedRequestBody<RegisterUserRequest>) -> impl IntoResponse {
        // ValidatedRequestBody 已经自动验证了参数，如果执行到这里，说明验证已通过
        ResponseEntity::ok(json!({
            "message": "用户注册成功",
            "username": request.username,
            "email": request.email,
            "age": request.age,
            "phone": request.phone,
            "note": "所有参数验证已通过"
        }))
    }

    /// POST /api/products
    /// 创建商品 - 演示更多验证规则
    ///
    /// 手动调用 validate() 方法验证商品信息
    ///
    /// 示例请求体：
    /// ```json
    /// {
    ///   "name": "MacBook Pro",
    ///   "description": "Apple M3 Max chip, 16-inch display",
    ///   "price": 19999,
    ///   "stock": 100
    /// }
    /// ```
    #[post_mapping("/products")]
    async fn create_product(&self, ValidatedRequestBody(request): ValidatedRequestBody<CreateProductRequest>) -> impl IntoResponse {
        // ValidatedRequestBody 已经自动验证了参数
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

    // ========== Cookie 和 Session 提取器示例 ==========

    /// GET /api/cookies
    /// 使用 Cookies 提取器获取所有 Cookie
    ///
    /// 示例请求：
    /// ```bash
    /// curl -X GET http://localhost:3000/api/cookies \
    ///   -H "Cookie: session_id=abc123; user_id=42; theme=dark"
    /// ```
    #[get_mapping("/cookies")]
    async fn get_cookies(&self, Cookies(cookies): Cookies) -> impl IntoResponse {
        let session_id = cookies.get("session_id").cloned();
        let user_id = cookies.get("user_id").cloned();
        let theme = cookies.get("theme").cloned();
        let total = cookies.len();

        ResponseEntity::ok(json!({
            "message": "Cookie 提取成功",
            "cookies": {
                "session_id": session_id,
                "user_id": user_id,
                "theme": theme,
                "total": total
            },
            "all_cookies": cookies,
            "note": "使用 Cookies 提取器获取所有 Cookie"
        }))
    }

    /// GET /api/session/user
    /// 使用 Session 提取器获取用户会话信息
    ///
    /// 示例请求（需要在 Cookie 中设置 session）：
    /// ```bash
    /// # 首先创建一个 JSON 格式的 session 数据
    /// # {"user_id":123,"username":"alice","role":"admin"}
    /// curl -X GET http://localhost:3000/api/session/user \
    ///   -H 'Cookie: session={"user_id":123,"username":"alice","role":"admin"}'
    /// ```
    #[get_mapping("/session/user")]
    async fn get_session_user(&self, Session(session): Session<serde_json::Value>) -> impl IntoResponse {
        ResponseEntity::ok(json!({
            "message": "Session 提取成功",
            "session_data": session,
            "note": "使用 Session 提取器从 Cookie 中提取会话信息（JSON 格式）"
        }))
    }

    /// GET /api/users/:id/preferences
    /// 组合使用 PathVariable 和 Cookies 提取器
    ///
    /// 示例请求：
    /// ```bash
    /// curl -X GET http://localhost:3000/api/users/42/preferences \
    ///   -H "Cookie: theme=dark; language=zh-CN; timezone=Asia/Shanghai"
    /// ```
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

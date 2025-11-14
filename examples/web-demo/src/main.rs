use chimera_core::prelude::*;
use chimera_core_macros::{Component, ConfigurationProperties};
use chimera_web_macros::{Controller, controller, get_mapping, post_mapping, put_mapping, request_mapping};
use chimera_web::prelude::*;
use chimera_web::exception_handler::ApplicationError;
// 明确导入提取器
use chimera_web::extractors::{Autowired, PathVariable, RequestBody, RequestParam, FormData, RequestHeaders};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// 导入我们的异常处理器模块
mod handlers {
    pub mod exception_handlers;
}

// ==================== 配置 ====================

#[derive(ConfigurationProperties, Debug, Clone)]
#[prefix("app")]
struct AppConfig {
    name: String,
    version: String,
}

// ==================== 数据模型 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreateUserRequest {
    name: String,
    email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpdateUserRequest {
    name: Option<String>,
    email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SearchQuery {
    name: Option<String>,
    email: Option<String>,
    page: Option<u32>,
    size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LoginForm {
    username: String,
    password: String,
    remember_me: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommentForm {
    author: String,
    content: String,
    rating: Option<u32>,
}

// ==================== 服务层 ====================

#[derive(Component, Clone)]
#[bean("userService")]
struct UserService {
    #[autowired]
    _config: Arc<AppConfig>,
}

impl UserService {
    fn list_users(&self) -> Vec<User> {
        vec![
            User {
                id: 1,
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
            },
            User {
                id: 2,
                name: "Bob".to_string(),
                email: "bob@example.com".to_string(),
            },
        ]
    }

    fn get_user_by_id(&self, id: u32) -> Option<User> {
        self.list_users().into_iter().find(|u| u.id == id)
    }

    fn create_user(&self, request: CreateUserRequest) -> User {
        User {
            id: 100,
            name: request.name,
            email: request.email,
        }
    }

    fn update_user(&self, id: u32, request: UpdateUserRequest) -> Option<User> {
        Some(User {
            id,
            name: request.name.unwrap_or_else(|| "Updated User".to_string()),
            email: request.email.unwrap_or_else(|| "updated@example.com".to_string()),
        })
    }

    fn search_users(&self, query: SearchQuery) -> Vec<User> {
        let mut users = self.list_users();

        if let Some(name) = query.name {
            users.retain(|u| u.name.contains(&name));
        }
        if let Some(email) = query.email {
            users.retain(|u| u.email.contains(&email));
        }

        users
    }
}

// ==================== 控制器 ====================
//
// 现在可以直接在 controller 方法中使用提取器！
// 框架会自动处理参数注入和路由注册

#[derive(Controller, Component, Clone)]
#[route("/api")]
struct ApiController {
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
        ResponseEntity::ok(serde_json::json!({
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
        ResponseEntity::ok(serde_json::json!({
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
            None => ResponseEntity::not_found(serde_json::json!({
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
            None => ResponseEntity::not_found(serde_json::json!({
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

    // ========== 使用 Autowired 注入其他服务 ==========

    /// GET /api/demo/autowired
    /// 演示在 handler 中使用 Autowired 注入其他服务
    #[get_mapping("/demo/autowired")]
    async fn demo_autowired(&self, Autowired(service): Autowired<UserService>) -> impl IntoResponse {
        // 这里的 service 是通过 Autowired 提取器注入的
        // 虽然 controller 本身已经有 user_service，但这展示了提取器的用法
        let users = service.list_users();
        ResponseEntity::ok(serde_json::json!({
            "message": "演示 Autowired 提取器",
            "users": users,
            "note": "service 参数是通过 Autowired<UserService> 提取器注入的"
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
        ResponseEntity::ok(serde_json::json!({
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
        ResponseEntity::ok(serde_json::json!({
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
        ResponseEntity::ok(serde_json::json!({
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

        ResponseEntity::ok(serde_json::json!({
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

        ResponseEntity::ok(serde_json::json!({
            "user_id": id,
            "authorization": authorization,
            "user_agent": user_agent,
            "note": "组合使用 PathVariable 和 RequestHeaders 提取器"
        }))
    }

    // ========== 测试异常处理器的端点 ==========

    /// GET /api/test/business-error
    /// 测试业务异常处理器
    #[get_mapping("/test/business-error")]
    async fn test_business_error(&self) -> impl IntoResponse {
        // 触发业务异常，会被 BusinessExceptionHandler 处理
        Err::<ResponseEntity<()>, _>(
            ApplicationError::ValidationError(
                "用户名必须长度在2-20个字符之间".to_string()
            )
        )
    }

    /// GET /api/test/database-error
    /// 测试数据库异常处理器
    #[get_mapping("/test/database-error")]
    async fn test_database_error(&self) -> impl IntoResponse {
        // 模拟数据库连接错误，会被 DatabaseExceptionHandler 处理
        Err::<ResponseEntity<()>, _>(
            ApplicationError::DatabaseError(
                "Database connection timeout after 30 seconds".to_string()
            )
        )
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
        ResponseEntity::ok(serde_json::json!({
            "message": "恭喜！您已成功通过身份验证",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "note": "这个端点需要有效的JWT token才能访问"
        }))
    }

    /// GET /api/admin/panel
    /// 需要管理员权限的端点
    #[get_mapping("/admin/panel")]
    async fn admin_panel(&self) -> impl IntoResponse {
        ResponseEntity::ok(serde_json::json!({
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

        ResponseEntity::ok(serde_json::json!({
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
        ResponseEntity::ok(serde_json::json!({
            "message": "这是一个公开端点，无需认证",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "note": "此端点在AuthInterceptor的排除列表中"
        }))
    }

    /// GET /api/test/rate-limit
    /// 测试限流拦截器的端点
    #[get_mapping("/test/rate-limit")]
    async fn test_rate_limit(&self) -> impl IntoResponse {
        ResponseEntity::ok(serde_json::json!({
            "message": "限流测试成功",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "note": "连续快速请求此端点以测试限流功能"
        }))
    }
}

// ==================== 文档控制器 ====================

#[derive(Controller, Component, Clone)]
#[route("/demo")]
struct DemoController;

#[controller]
impl DemoController {
    /// GET /demo/guide
    #[get_mapping("/guide")]
    async fn guide(&self) -> impl IntoResponse {
        ResponseEntity::ok(serde_json::json!({
            "title": "Chimera Web 参数注入完整指南",
            "description": "统一在 controller 方法中使用提取器进行参数注入",

            "philosophy": {
                "principle": "所有参数都通过提取器明确声明，直接在 controller 方法中使用",
                "benefits": [
                    "统一且清晰：参数来源一目了然",
                    "自动注册：无需手动注册路由",
                    "类型安全：编译时检查",
                    "Spring Boot 风格：完全符合 Spring Boot 使用习惯"
                ]
            },

            "available_extractors": {
                "autowired": {
                    "name": "Autowired<T>",
                    "description": "从 DI 容器注入 Bean（类似 @Autowired）",
                    "example": "Autowired(service): Autowired<UserService>",
                    "spring_boot": "@Autowired UserService userService"
                },
                "request_body": {
                    "name": "RequestBody<T>",
                    "description": "从 JSON body 反序列化（类似 @RequestBody）",
                    "example": "RequestBody(user): RequestBody<CreateUserRequest>",
                    "spring_boot": "@RequestBody User user"
                },
                "path_variable": {
                    "name": "PathVariable<T>",
                    "description": "从路径参数提取（类似 @PathVariable），支持正则验证",
                    "example": "PathVariable(id): PathVariable<u32>",
                    "validation": "path.validate(r\"^[a-zA-Z0-9_]+$\")",
                    "spring_boot": "@PathVariable Long id"
                },
                "request_param": {
                    "name": "RequestParam<T>",
                    "description": "从 query 参数反序列化（类似 @RequestParam）",
                    "example": "RequestParam(query): RequestParam<SearchQuery>",
                    "spring_boot": "@RequestParam String name"
                },
                "form_data": {
                    "name": "FormData<T>",
                    "description": "从表单数据反序列化（支持 application/x-www-form-urlencoded 和 multipart/form-data）",
                    "example": "FormData(form): FormData<LoginForm>",
                    "spring_boot": "@ModelAttribute LoginForm form"
                },
                "request_headers": {
                    "name": "RequestHeaders",
                    "description": "提取所有 HTTP 请求头（类似 @RequestHeader）",
                    "example": "RequestHeaders(headers): RequestHeaders",
                    "spring_boot": "@RequestHeader HttpHeaders headers"
                }
            },

            "usage_examples": {
                "simple": {
                    "description": "获取单个用户",
                    "code": "#[get_mapping(\"/users/:id\")] async fn get_user(&self, PathVariable(id): PathVariable<u32>) -> impl IntoResponse"
                },
                "with_body": {
                    "description": "创建用户",
                    "code": "#[post_mapping(\"/users\")] async fn create_user(&self, RequestBody(req): RequestBody<CreateUserRequest>) -> impl IntoResponse"
                },
                "combined": {
                    "description": "更新用户（组合路径参数和请求体）",
                    "code": "#[put_mapping(\"/users/:id\")] async fn update_user(&self, PathVariable(id): PathVariable<u32>, RequestBody(req): RequestBody<UpdateRequest>) -> impl IntoResponse"
                },
                "complex": {
                    "description": "复杂操作（三种提取器组合）",
                    "code": "#[post_mapping(\"/users/:id/actions\")] async fn action(&self, PathVariable(id): PathVariable<u32>, RequestParam(params): RequestParam<Value>, RequestBody(body): RequestBody<Value>) -> impl IntoResponse"
                }
            },

            "comparison_with_spring_boot": {
                "spring_boot": {
                    "controller": "@RestController @RequestMapping(\"/api\")",
                    "autowired": "@Autowired UserService userService",
                    "request_body": "@RequestBody User user",
                    "path_variable": "@PathVariable Long id",
                    "request_param": "@RequestParam String name"
                },
                "chimera": {
                    "controller": "#[derive(Controller)] #[route(\"/api\")]",
                    "autowired": "Autowired(userService): Autowired<UserService>",
                    "request_body": "RequestBody(user): RequestBody<User>",
                    "path_variable": "PathVariable(id): PathVariable<u32>",
                    "request_param": "RequestParam(name): RequestParam<String>"
                }
            },

            "key_features": [
                "完全自动化：路由自动注册，无需手动配置",
                "类型安全：所有参数都有明确的类型",
                "错误处理：提取失败自动返回适当的 HTTP 状态码",
                "灵活组合：可以在一个方法中使用多个提取器",
                "Spring Boot 风格：完全符合 Java 开发者的使用习惯"
            ]
        }))
    }
}

// ==================== 主程序 ====================

#[tokio::main]
async fn main() -> ApplicationResult<()> {
    // 配置文件会自动从以下位置查找（按优先级）：
    // 1. config/application.toml
    // 2. application.toml
    // 也可以手动指定：.config_file("custom/path/to/config.toml")

    // 一行启动应用并阻塞（类似 Spring Boot 的 SpringApplication.run()）
    ChimeraApplication::new("WebDemo")
        .env_prefix("WEB_")
        .run_until_shutdown()
        .await
}

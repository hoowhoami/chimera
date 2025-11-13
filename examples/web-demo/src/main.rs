use chimera_core::prelude::*;
use chimera_core_macros::{Component, ConfigurationProperties};
use chimera_web_macros::{Controller, controller, get_mapping, post_mapping, put_mapping, request_mapping};
use chimera_web::prelude::*;
// 明确导入提取器
use chimera_web::extractors::{Autowired, PathVariable, RequestBody, RequestParam};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
                    "description": "从路径参数提取（类似 @PathVariable）",
                    "example": "PathVariable(id): PathVariable<u32>",
                    "spring_boot": "@PathVariable Long id"
                },
                "request_param": {
                    "name": "RequestParam<T>",
                    "description": "从 query 参数反序列化（类似 @RequestParam）",
                    "example": "RequestParam(query): RequestParam<SearchQuery>",
                    "spring_boot": "@RequestParam String name"
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
                "✅ 完全自动化：路由自动注册，无需手动配置",
                "✅ 类型安全：所有参数都有明确的类型",
                "✅ 错误处理：提取失败自动返回适当的 HTTP 状态码",
                "✅ 灵活组合：可以在一个方法中使用多个提取器",
                "✅ Spring Boot 风格：完全符合 Java 开发者的使用习惯"
            ]
        }))
    }
}

// ==================== 主程序 ====================

#[tokio::main]
async fn main() -> ApplicationResult<()> {
    println!("🌐 Chimera Web - Parameter Injection Demo");
    println!("==========================================\n");
    println!("✨ 现在可以直接在 controller 方法中使用提取器！\n");
    println!("核心特性：");
    println!("  ✓ 自动路由注册 - 无需手动配置");
    println!("  ✓ Spring Boot 风格 - Autowired, RequestBody, PathVariable, RequestParam");
    println!("  ✓ 类型安全 - 编译时检查所有参数");
    println!("  ✓ 灵活组合 - 在一个方法中使用多个提取器\n");

    let config_file = if std::path::Path::new("examples/web-demo/application.toml").exists() {
        "examples/web-demo/application.toml"
    } else {
        "application.toml"
    };

    let app = ChimeraApplication::new("WebDemo")
        .config_file(config_file)
        .env_prefix("WEB_")
        .run()
        .await?;

    println!("\n📋 可用的 API 端点：\n");
    println!("  【基础路由】");
    println!("  GET    /api/info              - 应用信息");
    println!("  GET    /api/users             - 用户列表");
    println!("  *      /api/health            - 健康检查\n");

    println!("  【PathVariable 示例】");
    println!("  GET    /api/users/:id         - 获取单个用户\n");

    println!("  【RequestBody 示例】");
    println!("  POST   /api/users/create      - 创建用户\n");

    println!("  【组合示例】");
    println!("  PUT    /api/users/:id         - 更新用户（PathVariable + RequestBody）");
    println!("  GET    /api/users/search      - 搜索用户（RequestParam）");
    println!("  POST   /api/users/:id/actions - 复杂操作（三种提取器组合）\n");

    println!("  【Autowired 示例】");
    println!("  GET    /api/demo/autowired    - 演示 Autowired 提取器\n");

    println!("  【文档】");
    println!("  GET    /demo/guide            - 完整使用指南\n");

    println!("💡 所有路由都已自动注册，无需手动配置！\n");

    app.wait_for_shutdown().await?;

    Ok(())
}

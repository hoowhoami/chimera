use chimera_core::prelude::*;
use chimera_core_macros::{Component, ConfigurationProperties};
use chimera_web_macros::{Controller, controller, get_mapping, request_mapping};
use chimera_web::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ==================== 配置 ====================

#[derive(ConfigurationProperties, Debug, Clone)]
#[prefix("app")]
struct AppConfig {
    name: String,
    version: String,
}

// ==================== 服务层 ====================

#[derive(Component, Clone)]
#[bean("userService")]
struct UserService {
    #[autowired]
    config: Arc<AppConfig>,
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
}

// ==================== 数据模型 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

// ==================== 控制器 ====================

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
    #[get_mapping("/info")]
    async fn get_info(&self) -> impl IntoResponse {
        ResponseEntity::ok(serde_json::json!({
            "app": self.config.name,
            "version": self.config.version,
            "status": "running"
        }))
    }

    #[get_mapping("/users")]
    async fn list_users(&self) -> impl IntoResponse {
        let users = self.user_service.list_users();
        ResponseEntity::ok(users)
    }

    // 带路径参数的路由 - 获取单个用户
    #[get_mapping("/users/:id")]
    async fn get_user(&self, id: String) -> impl IntoResponse {
        match id.parse::<u32>() {
            Ok(user_id) => {
                match self.user_service.get_user_by_id(user_id) {
                    Some(user) => ResponseEntity::ok(serde_json::json!(user)),
                    None => ResponseEntity::not_found(serde_json::json!({
                        "error": "User not found",
                        "id": user_id
                    }))
                }
            }
            Err(_) => ResponseEntity::bad_request(serde_json::json!({
                "error": "Invalid user ID",
                "id": id
            }))
        }
    }

    // 路径参数
    #[get_mapping("/test/:id")]
    async fn test_path_param(&self, id: u32) -> impl IntoResponse {
        ResponseEntity::ok(serde_json::json!({
            "message": "test path param",
            "number": id,
            "doubled": id * 2
        }))
    }

    // 带正则验证的路径参数 - 只接受数字ID
    #[get_mapping("/users/:id<^\\d+$>/profile")]
    async fn get_user_profile(&self, id: u32) -> impl IntoResponse {
        // id 现在直接是 u32 类型，框架已经自动完成了解析和验证
        match self.user_service.get_user_by_id(id) {
            Some(user) => ResponseEntity::ok(serde_json::json!({
                "profile": {
                    "user": user,
                    "bio": format!("Profile of {}", user.name),
                    "member_since": "2024-01-01"
                }
            })),
            None => ResponseEntity::not_found(serde_json::json!({
                "error": "User not found"
            }))
        }
    }

    // 演示通用路由 - 接受所有 HTTP 方法
    #[request_mapping("/health")]
    async fn health_check(&self) -> impl IntoResponse {
        ResponseEntity::ok(serde_json::json!({
            "status": "healthy",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}

// ==================== 主程序 ====================

#[tokio::main]
async fn main() -> ApplicationResult<()> {
    println!("🌐 Chimera Web - Controller Demo\n");

    // 配置文件路径
    let config_file = if std::path::Path::new("examples/web-demo/application.toml").exists() {
        "examples/web-demo/application.toml"
    } else {
        "application.toml"
    };

    // ✨ 只需要一行启动代码！
    // Web 服务器和所有控制器会自动配置和启动
    ChimeraApplication::new("WebDemo")
        .config_file(config_file)
        .env_prefix("WEB_")
        .run()
        .await?
        .wait_for_shutdown()
        .await?;

    Ok(())
}

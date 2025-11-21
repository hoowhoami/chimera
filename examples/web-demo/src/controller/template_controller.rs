use crate::models::RegisterUserRequest;
use crate::service::UserService;
use chimera_core::prelude::*;
use chimera_core_macros::{component, Component};
use chimera_web::prelude::*;
use chimera_web::template::Template;
use chimera_web_macros::{controller, get_mapping};
use std::sync::Arc;
use serde::Serialize;

/// 模板控制器
///
/// 展示如何使用 Tera 模板引擎渲染 HTML 页面
/// 类似 Spring Boot 的 Thymeleaf 模板引擎
#[controller("/templates")]
#[derive(Component, Clone)]
pub struct TemplateController {
    #[autowired]
    user_service: Arc<UserService>,
}

/// 用户视图模型
#[derive(Debug, Clone, Serialize)]
struct UserViewModel {
    id: u32,
    username: String,
    email: String,
    age: u32,
    status: String,
}

#[component]
#[controller]
impl TemplateController {
    /// 首页 - 基础模板渲染示例
    ///
    /// 访问: GET /templates/home
    #[get_mapping("/home")]
    async fn home(&self) -> impl IntoResponse {
        Template::new("index.html")
            .with("title", "Chimera Web Framework")
            .with("message", "欢迎使用 Chimera Web 框架！这是一个基于 Rust 的现代化 Web 框架。")
            .with("timestamp", chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string())
    }

    /// 用户列表页面 - 展示如何传递集合数据
    ///
    /// 访问: GET /templates/users
    #[get_mapping("/users")]
    async fn user_list(&self) -> impl IntoResponse {
        // 从服务层获取用户数据
        let users = vec![
            UserViewModel {
                id: 1,
                username: "alice".to_string(),
                email: "alice@example.com".to_string(),
                age: 28,
                status: "active".to_string(),
            },
            UserViewModel {
                id: 2,
                username: "bob".to_string(),
                email: "bob@example.com".to_string(),
                age: 32,
                status: "active".to_string(),
            },
            UserViewModel {
                id: 3,
                username: "charlie".to_string(),
                email: "charlie@example.com".to_string(),
                age: 25,
                status: "inactive".to_string(),
            },
        ];

        Template::new("users.html")
            .with("title", "用户列表")
            .with("users", users)
            .with("total", 3)
            .with("timestamp", chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string())
    }

    /// 用户详情页面 - 展示如何使用路径参数
    ///
    /// 访问: GET /templates/user/1
    #[get_mapping("/user/:id")]
    async fn user_detail(&self, PathVariable(id): PathVariable<u32>) -> impl IntoResponse {
        // 模拟从数据库获取用户
        let user = UserViewModel {
            id,
            username: format!("user_{}", id),
            email: format!("user_{}@example.com", id),
            age: 20 + id,
            status: "active".to_string(),
        };

        Template::new("user_detail.html")
            .with("title", format!("用户详情 - {}", user.username))
            .with("user", user)
            .with("timestamp", chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string())
    }

    /// 关于页面 - 展示静态内容渲染
    ///
    /// 访问: GET /templates/about
    #[get_mapping("/about")]
    async fn about(&self) -> impl IntoResponse {
        Template::new("about.html")
            .with("title", "关于 Chimera")
            .with("version", "1.0.0")
            .with("description", "Chimera 是一个受 Spring Boot 启发的 Rust Web 框架")
            .with("features", vec![
                "依赖注入 (DI)",
                "自动装配 (AutoConfiguration)",
                "注解驱动开发",
                "模板引擎支持",
                "热重载开发",
                "异常处理",
                "数据验证",
            ])
            .with("timestamp", chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string())
    }

    /// 表单页面 - 展示表单渲染
    ///
    /// 访问: GET /templates/form
    #[get_mapping("/form")]
    async fn show_form(&self) -> impl IntoResponse {
        Template::new("form.html")
            .with("title", "用户注册表单")
            .with("action", "/user/register")
            .with("method", "POST")
            .with("timestamp", chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string())
    }

    /// 错误页面示例 - 展示如何设置 HTTP 状态码
    ///
    /// 访问: GET /templates/error
    #[get_mapping("/error")]
    async fn error_page(&self) -> impl IntoResponse {
        Template::new("error.html")
            .with("title", "页面未找到")
            .with("error_code", 404)
            .with("error_message", "您访问的页面不存在")
            .with("timestamp", chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string())
            .status(StatusCode::NOT_FOUND)
    }

    /// 条件渲染示例 - 展示如何使用条件变量
    ///
    /// 访问: GET /templates/conditional?logged_in=true
    #[get_mapping("/conditional")]
    async fn conditional(&self, Query(params): Query<std::collections::HashMap<String, String>>) -> impl IntoResponse {
        let logged_in = params.get("logged_in")
            .map(|v| v == "true")
            .unwrap_or(false);

        let username = if logged_in {
            Some("admin")
        } else {
            None
        };

        Template::new("conditional.html")
            .with("title", "条件渲染示例")
            .with("logged_in", logged_in)
            .with("username", username)
            .with("timestamp", chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string())
    }

    /// 嵌套数据示例 - 展示如何传递复杂的嵌套数据结构
    ///
    /// 访问: GET /templates/nested
    #[get_mapping("/nested")]
    async fn nested_data(&self) -> impl IntoResponse {
        #[derive(Serialize)]
        struct Department {
            name: String,
            employees: Vec<UserViewModel>,
        }

        let departments = vec![
            Department {
                name: "工程部".to_string(),
                employees: vec![
                    UserViewModel {
                        id: 1,
                        username: "alice".to_string(),
                        email: "alice@example.com".to_string(),
                        age: 28,
                        status: "active".to_string(),
                    },
                    UserViewModel {
                        id: 2,
                        username: "bob".to_string(),
                        email: "bob@example.com".to_string(),
                        age: 32,
                        status: "active".to_string(),
                    },
                ],
            },
            Department {
                name: "市场部".to_string(),
                employees: vec![
                    UserViewModel {
                        id: 3,
                        username: "charlie".to_string(),
                        email: "charlie@example.com".to_string(),
                        age: 25,
                        status: "active".to_string(),
                    },
                ],
            },
        ];

        Template::new("nested.html")
            .with("title", "嵌套数据示例")
            .with("departments", departments)
            .with("timestamp", chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string())
    }
}


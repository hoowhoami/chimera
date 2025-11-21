//! 模板引擎支持
//!
//! 基于 Tera 模板引擎提供类似 Spring Boot Thymeleaf 的模板渲染功能
//!
//! ## 使用示例
//!
//! ```ignore
//! use chimera_web::prelude::*;
//! use serde_json::json;
//!
//! #[get_mapping("/hello")]
//! async fn hello(&self) -> impl IntoResponse {
//!     Template::new("hello.html")
//!         .with("name", "World")
//!         .with("message", "Welcome to Chimera!")
//! }
//! ```

use axum::response::{IntoResponse, Response, Html};
use axum::http::StatusCode;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::RwLock;
use tera::Tera;
use tokio::sync::broadcast;
use notify::{Watcher, RecommendedWatcher, RecursiveMode, Event};
use std::path::Path;

use crate::constants::*;

/// Tera 模板引擎配置
///
/// 用于初始化和配置 Tera 模板引擎
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateProperties {
    /// 是否启用 Tera 模板引擎
    pub enabled: bool,

    /// Tera 模板目录（默认 "templates"）
    pub template_dir: String,

    /// Tera 模板模式（默认 "templates/**/*"）
    pub pattern: String,

    /// 是否启用 Tera 热加载（默认 false）
    pub hot_reload: bool,
}

impl Default for TemplateProperties {
    fn default() -> Self {
        Self {
            enabled: false,
            template_dir: "templates".to_string(),
            pattern: "templates/**/*".to_string(),
            hot_reload: false,
        }
    }
}

impl TemplateProperties {
    /// 从 Environment 加载配置
    pub fn from_environment(env: &chimera_core::prelude::Environment) -> Self {
        Self {
            enabled: env.get_bool(TERA_ENABLED).unwrap_or(false),
            template_dir: env
                .get_string(TERA_TEMPLATE_DIR)
                .unwrap_or_else(|| "templates".to_string()),
            pattern: env
                .get_string(TERA_PATTERN)
                .unwrap_or_else(|| "templates/**/*".to_string()),
            hot_reload: env.get_bool(TERA_HOT_RELOAD).unwrap_or(false),
        }
    }
}

/// 模板响应 - 类似 Spring Boot 的 ModelAndView
///
/// 用于在控制器中返回 HTML 模板响应
///
/// ## 示例
///
/// ```ignore
/// #[get_mapping("/user/{id}")]
/// async fn get_user(&self, PathVariable(id): PathVariable<u32>, engine: TemplateEngine) -> impl IntoResponse {
///     let user = self.user_service.find_by_id(id).await?;
///
///     engine.render("user/detail.html")
///         .with("user", user)
///         .with("title", format!("User #{}", id))
/// }
/// ```
pub struct Template {
    /// 模板名称（相对于模板目录的路径）
    template_name: String,
    /// 模板上下文数据
    context: tera::Context,
    /// HTTP 状态码（默认 200）
    status: StatusCode,
    /// 模板引擎引用
    engine: TemplateEngine,
}

impl Template {

    /// 添加单个变量到模板上下文
    ///
    /// # 参数
    ///
    /// * `key` - 变量名
    /// * `value` - 变量值（需实现 Serialize）
    ///
    /// # 示例
    ///
    /// ```ignore
    /// Template::new("hello.html")
    ///     .with("name", "Alice")
    ///     .with("age", 30)
    /// ```
    pub fn with<K: Into<String>, V: Serialize>(mut self, key: K, value: V) -> Self {
        self.context.insert(key, &value);
        self
    }

    /// 批量添加多个变量到模板上下文
    ///
    /// # 参数
    ///
    /// * `data` - 包含多个键值对的 HashMap
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let mut data = HashMap::new();
    /// data.insert("name", "Alice");
    /// data.insert("email", "alice@example.com");
    ///
    /// Template::new("profile.html")
    ///     .with_data(data)
    /// ```
    pub fn with_data<K: Into<String>, V: Serialize>(
        mut self,
        data: HashMap<K, V>,
    ) -> Self {
        for (key, value) in data {
            self.context.insert(key, &value);
        }
        self
    }

    /// 设置 HTTP 状态码
    ///
    /// # 示例
    ///
    /// ```ignore
    /// Template::new("error.html")
    ///     .with("error", "Not Found")
    ///     .status(StatusCode::NOT_FOUND)
    /// ```
    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

}

impl IntoResponse for Template {
    fn into_response(self) -> Response {
        match self.engine.render_internal(&self.template_name, &self.context) {
            Ok(html) => (self.status, Html(html)).into_response(),
            Err(err) => {
                tracing::error!(error = ?err, template = %self.template_name, "Template render error");

                // 返回错误响应
                let error_html = format!(
                    r#"<!DOCTYPE html>
<html>
<head>
    <title>Template Error</title>
</head>
<body>
    <h1>Template Rendering Error</h1>
    <p><strong>Template:</strong> {}</p>
    <p><strong>Error:</strong> {}</p>
</body>
</html>"#,
                    self.template_name, err
                );

                (StatusCode::INTERNAL_SERVER_ERROR, Html(error_html)).into_response()
            }
        }
    }
}

/// 模板错误类型
#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    #[error("Template engine not initialized. Call TemplateEngine::init() first.")]
    EngineNotInitialized,

    #[error("Failed to acquire template engine lock")]
    EngineLockError,

    #[error("Failed to render template '{template}': {cause}")]
    RenderError { template: String, cause: String },

    #[error("Template initialization error: {0}")]
    InitError(String),
}

/// 模板引擎 - 可注入的 Bean
///
/// 用于管理 Tera 模板引擎实例
#[derive(Clone)]
pub struct TemplateEngine {
    tera: std::sync::Arc<RwLock<Tera>>,
    hot_reload_tx: Option<std::sync::Arc<broadcast::Sender<()>>>,
}

/// 热重载配置
#[derive(Debug, Clone)]
pub struct HotReloadConfig {
    pub enabled: bool,
    pub debounce_ms: u64,
    pub watch_pattern: String,
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            debounce_ms: 1000,
            watch_pattern: "templates/**/*".to_string(),
        }
    }
}

impl TemplateEngine {
    /// 创建新的模板引擎实例
    ///
    /// # 参数
    ///
    /// * `pattern` - glob 模式，例如 "templates/**/*.html"
    /// * `hot_reload` - 是否启用热重载
    pub fn new(pattern: &str, hot_reload: bool) -> Result<Self, TemplateError> {
        let tera = Tera::new(pattern).map_err(|e| {
            TemplateError::InitError(format!("Failed to initialize Tera with pattern '{}': {}", pattern, e))
        })?;

        tracing::info!(pattern = %pattern, hot_reload = %hot_reload, "Template engine created");

        Ok(Self {
            tera: std::sync::Arc::new(RwLock::new(tera)),
            hot_reload_tx: None,
        })
    }

    /// 从 Environment 创建模板引擎
    pub fn from_environment(env: &chimera_core::prelude::Environment) -> Result<Self, TemplateError> {
        let props = TemplateProperties::from_environment(env);

        if !props.enabled {
            return Err(TemplateError::InitError("Template engine is disabled in configuration".to_string()));
        }

        let mut engine = Self::new(&props.pattern, props.hot_reload)?;

        // 如果启用热重载，启动监听
        if props.hot_reload {
            engine.start_hot_reload(&props.pattern)?;
        }

        Ok(engine)
    }

    /// 创建模板响应
    ///
    /// # 参数
    ///
    /// * `template_name` - 模板文件名（相对于模板目录）
    ///
    /// # 示例
    ///
    /// ```ignore
    /// engine.render("index.html")
    ///     .with("name", "Alice")
    ///     .with("age", 30)
    /// ```
    pub fn render(&self, template_name: impl Into<String>) -> Template {
        Template {
            template_name: template_name.into(),
            context: tera::Context::new(),
            status: StatusCode::OK,
            engine: self.clone(),
        }
    }

    /// 内部渲染方法
    fn render_internal(&self, template_name: &str, context: &tera::Context) -> Result<String, TemplateError> {
        self.tera
            .read()
            .map_err(|_| TemplateError::EngineLockError)?
            .render(template_name, context)
            .map_err(|e| TemplateError::RenderError {
                template: template_name.to_string(),
                cause: e.to_string(),
            })
    }

    /// 启动热重载监听
    fn start_hot_reload(&mut self, pattern: &str) -> Result<(), TemplateError> {
        let (tx, mut rx) = broadcast::channel(1);
        self.hot_reload_tx = Some(std::sync::Arc::new(tx.clone()));

        let tera_arc = self.tera.clone();

        let watch_path = pattern.trim_end_matches("/**/*").to_string();

        tokio::spawn(async move {
            let mut watcher = RecommendedWatcher::new(
                move |res: Result<Event, notify::Error>| {
                    match res {
                        Ok(event) => {
                            tracing::debug!(event = ?event, "Template file changed");
                            // 发送重载信号
                            let _ = tx.send(());
                        }
                        Err(e) => {
                            tracing::error!(error = ?e, "File watching error");
                        }
                    }
                },
                notify::Config::default().with_poll_interval(std::time::Duration::from_millis(1000)),
            ).expect("Failed to create watcher");

            // 开始监听模板目录
            if let Err(e) = watcher.watch(Path::new(&watch_path), RecursiveMode::Recursive) {
                tracing::error!(error = ?e, path = %watch_path, "Failed to watch template directory");
                return;
            }

            tracing::info!(path = %watch_path, "Hot reload started");

            // 使用防抖动机制处理重载信号
            let mut debounce_timer = tokio::time::interval(std::time::Duration::from_millis(500));
            debounce_timer.tick().await; // 等待第一次tick

            loop {
                tokio::select! {
                    _ = rx.recv() => {
                        // 收到重载信号，延迟执行
                        debounce_timer.tick().await;
                        tracing::info!("Reloading templates...");

                        // 重新加载模板
                        if let Ok(mut tera) = tera_arc.write() {
                            if let Err(e) = tera.full_reload() {
                                tracing::error!(error = ?e, "Template reload failed");
                            } else {
                                tracing::info!("Templates reloaded");
                            }
                        }
                    }
                    _ = debounce_timer.tick() => {
                        // 定期检查
                    }
                }
            }
        });

        Ok(())
    }

    /// 检查热重载是否已启用
    pub fn is_hot_reload_enabled(&self) -> bool {
        self.hot_reload_tx.is_some()
    }

}

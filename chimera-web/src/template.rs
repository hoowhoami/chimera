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

    /// Tera 全局变量映射
    pub globals: HashMap<String, String>,
}

impl Default for TemplateProperties {
    fn default() -> Self {
        Self {
            enabled: false,
            template_dir: "templates".to_string(),
            pattern: "templates/**/*".to_string(),
            hot_reload: false,
            globals: HashMap::new(),
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
            globals: HashMap::new(), // TODO: Implement proper HashMap configuration support
        }
    }
}

/// 模板配置扩展
impl TemplateEngine {
    /// 从 Environment 加载完整配置并初始化
    pub fn init_with_config(env: &chimera_core::prelude::Environment) -> Result<(), TemplateError> {
        let props = TemplateProperties::from_environment(env);

        if props.enabled {
            // 使用配置的模式初始化
            Self::init_with_pattern(&props.pattern)?;

            // 如果热重载开启，启动监听
            if props.hot_reload {
                Self::start_hot_reload(&props.pattern)?;
            }
        }

        Ok(())
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
/// async fn get_user(&self, PathVariable(id): PathVariable<u32>) -> impl IntoResponse {
///     let user = self.user_service.find_by_id(id).await?;
///
///     Template::new("user/detail.html")
///         .with("user", user)
///         .with("title", format!("User #{}", id))
/// }
/// ```
#[derive(Clone)]
pub struct Template {
    /// 模板名称（相对于模板目录的路径）
    template_name: String,
    /// 模板上下文数据
    context: tera::Context,
    /// HTTP 状态码（默认 200）
    status: StatusCode,
}

impl Template {
    /// 创建新的模板响应
    ///
    /// # 参数
    ///
    /// * `template_name` - 模板文件名（相对于模板目录）
    ///
    /// # 示例
    ///
    /// ```ignore
    /// Template::new("index.html")
    /// Template::new("user/list.html")
    /// ```
    pub fn new(template_name: impl Into<String>) -> Self {
        Self {
            template_name: template_name.into(),
            context: tera::Context::new(),
            status: StatusCode::OK,
        }
    }

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

    /// 渲染模板
    ///
    /// 使用全局模板引擎渲染此模板
    fn render(&self) -> Result<String, TemplateError> {
        GLOBAL_TERA
            .read()
            .map_err(|_| TemplateError::EngineLockError)?
            .as_ref()
            .ok_or(TemplateError::EngineNotInitialized)?
            .render(&self.template_name, &self.context)
            .map_err(|e| TemplateError::RenderError {
                template: self.template_name.clone(),
                cause: e.to_string(),
            })
    }
}

impl IntoResponse for Template {
    fn into_response(self) -> Response {
        match self.render() {
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

/// 全局模板引擎实例
static GLOBAL_TERA: RwLock<Option<Tera>> = RwLock::new(None);

/// 热重载广播通道
static HOT_RELOAD_TX: RwLock<Option<broadcast::Sender<()>>> = RwLock::new(None);

/// 模板引擎配置
///
/// 用于初始化和配置 Tera 模板引擎
pub struct TemplateEngine;

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
    /// 使用默认配置初始化模板引擎
    ///
    /// 默认从 `templates/**/*` 加载所有模板文件
    ///
    /// # 示例
    ///
    /// ```ignore
    /// TemplateEngine::init()?;
    /// ```
    pub fn init() -> Result<(), TemplateError> {
        Self::init_with_pattern("templates/**/*")
    }

    /// 使用自定义 glob 模式初始化模板引擎
    ///
    /// # 参数
    ///
    /// * `pattern` - glob 模式，例如 "templates/**/*.html"
    ///
    /// # 示例
    ///
    /// ```ignore
    /// TemplateEngine::init_with_pattern("views/**/*.html")?;
    /// ```
    pub fn init_with_pattern(pattern: &str) -> Result<(), TemplateError> {
        let tera = Tera::new(pattern).map_err(|e| {
            TemplateError::InitError(format!("Failed to initialize Tera with pattern '{}': {}", pattern, e))
        })?;

        *GLOBAL_TERA
            .write()
            .map_err(|_| TemplateError::EngineLockError)? = Some(tera);

        tracing::info!(pattern = %pattern, "Template engine initialized");
        Ok(())
    }

    /// 使用自定义 Tera 实例初始化模板引擎
    ///
    /// 这允许你完全自定义 Tera 配置
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let mut tera = Tera::default();
    /// tera.add_raw_template("hello.html", "<h1>Hello {{ name }}!</h1>")?;
    /// TemplateEngine::init_with_tera(tera)?;
    /// ```
    pub fn init_with_tera(tera: Tera) -> Result<(), TemplateError> {
        *GLOBAL_TERA
            .write()
            .map_err(|_| TemplateError::EngineLockError)? = Some(tera);

        tracing::info!("Template engine initialized with custom Tera instance");
        Ok(())
    }

    /// 获取全局 Tera 实例的只读引用
    ///
    /// 用于高级场景，需要直接访问 Tera API
    ///
    /// # 示例
    ///
    /// ```ignore
    /// TemplateEngine::with_tera(|tera| {
    ///     // 使用 tera 进行自定义操作
    ///     println!("Available templates: {:?}", tera.get_template_names());
    /// })?;
    /// ```
    pub fn with_tera<F, R>(f: F) -> Result<R, TemplateError>
    where
        F: FnOnce(&Tera) -> R,
    {
        let guard = GLOBAL_TERA
            .read()
            .map_err(|_| TemplateError::EngineLockError)?;

        let tera = guard
            .as_ref()
            .ok_or(TemplateError::EngineNotInitialized)?;

        Ok(f(tera))
    }



    /// 启动热重载监听
    fn start_hot_reload(pattern: &str) -> Result<(), TemplateError> {
        let (tx, mut rx) = broadcast::channel(1);
        *HOT_RELOAD_TX.write().map_err(|_| TemplateError::EngineLockError)? = Some(tx.clone());

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

                        if let Err(e) = Self::reload() {
                            tracing::error!(error = ?e, "Template reload failed");
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
    pub fn is_hot_reload_enabled() -> bool {
        HOT_RELOAD_TX.read().ok().map(|guard| guard.is_some()).unwrap_or(false)
    }

    /// 停止热重载监听
    pub fn stop_hot_reload() -> Result<(), TemplateError> {
        *HOT_RELOAD_TX.write().map_err(|_| TemplateError::EngineLockError)? = None;
        tracing::info!("Hot reload stopped");
        Ok(())
    }

    /// 重新加载所有模板
    ///
    /// 在开发模式下很有用，可以热重载模板变更
    ///
    /// # 示例
    ///
    /// ```ignore
    /// TemplateEngine::reload()?;
    /// ```
    pub fn reload() -> Result<(), TemplateError> {
        let mut guard = GLOBAL_TERA
            .write()
            .map_err(|_| TemplateError::EngineLockError)?;

        if let Some(tera) = guard.as_mut() {
            tera.full_reload().map_err(|e| {
                TemplateError::InitError(format!("Failed to reload templates: {}", e))
            })?;

            tracing::info!("Templates reloaded");
            Ok(())
        } else {
            Err(TemplateError::EngineNotInitialized)
        }
    }
}

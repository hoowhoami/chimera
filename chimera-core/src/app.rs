use crate::{ApplicationContext, ApplicationResult};
use crate::config::{TomlPropertySource, EnvironmentPropertySource};
use crate::event::ApplicationStartedEvent;
use crate::logging::LoggingConfig;
use crate::error::ContainerResult;
use crate::plugin::{PluginRegistry, load_plugins};
use std::sync::Arc;
use std::path::Path;

/// 正在运行的应用
///
/// 包装 ApplicationContext 并提供额外的生命周期管理方法
pub struct RunningApplication {
    context: Arc<ApplicationContext>,
}

impl RunningApplication {
    /// 获取 ApplicationContext 引用
    pub fn context(&self) -> &Arc<ApplicationContext> {
        &self.context
    }

    /// 转换为 ApplicationContext（消费 self）
    pub fn into_context(self) -> Arc<ApplicationContext> {
        self.context
    }

    /// 等待应用关闭
    ///
    /// 此方法会阻塞当前线程，直到接收到关闭信号
    pub async fn wait_for_shutdown(self) -> ApplicationResult<()> {
        // 阻塞直到程序被信号中断
        let () = std::future::pending().await;
        Ok(())
    }

    /// 手动触发关闭
    pub async fn shutdown(self) -> ApplicationResult<()> {
        self.context.shutdown().await.map_err(|e| e.into())
    }
}

// 实现 Deref 以便可以直接调用 ApplicationContext 的方法
impl std::ops::Deref for RunningApplication {
    type Target = Arc<ApplicationContext>;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

/// Chimera 应用程序
///
/// 提供便捷的应用启动方式
pub struct ChimeraApplication {
    /// 应用名称
    name: String,

    /// 配置文件路径
    config_files: Vec<String>,

    /// 环境变量前缀
    env_prefix: String,

    /// 激活的 profiles
    profiles: Vec<String>,

    /// 是否显示 banner
    show_banner: bool,

    /// 日志配置
    logging_config: Option<LoggingConfig>,

    /// 自定义初始化函数
    initializers: Vec<Box<dyn Fn(&Arc<ApplicationContext>) -> ApplicationResult<()> + Send + Sync>>,

    /// 自定义 shutdown hooks
    shutdown_hooks: Vec<Box<dyn Fn() -> ContainerResult<()> + Send + Sync>>,

    /// 插件注册表
    plugin_registry: PluginRegistry,
}

impl ChimeraApplication {
    /// 创建新的应用
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            config_files: Vec::new(), // 初始为空，将在 run 时根据规则查找
            env_prefix: "APP_".to_string(),
            profiles: Vec::new(),
            show_banner: true,
            logging_config: None,
            initializers: Vec::new(),
            shutdown_hooks: Vec::new(),
            plugin_registry: load_plugins(), // 自动加载所有插件
        }
    }

    /// 设置配置文件路径
    pub fn config_file(mut self, path: impl Into<String>) -> Self {
        self.config_files = vec![path.into()];
        self
    }

    /// 添加多个配置文件
    pub fn config_files(mut self, paths: Vec<String>) -> Self {
        self.config_files = paths;
        self
    }

    /// 设置环境变量前缀
    pub fn env_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.env_prefix = prefix.into();
        self
    }

    /// 设置激活的 profiles
    pub fn profiles(mut self, profiles: Vec<String>) -> Self {
        self.profiles = profiles;
        self
    }

    /// 设置是否显示 banner
    pub fn banner(mut self, show: bool) -> Self {
        self.show_banner = show;
        self
    }

    /// 设置日志配置
    ///
    /// 如果不设置，将使用默认配置（从环境变量读取）
    pub fn logging(mut self, config: LoggingConfig) -> Self {
        self.logging_config = Some(config);
        self
    }

    /// 添加初始化器
    pub fn initializer<F>(mut self, f: F) -> Self
    where
        F: Fn(&Arc<ApplicationContext>) -> ApplicationResult<()> + Send + Sync + 'static,
    {
        self.initializers.push(Box::new(f));
        self
    }

    /// 添加 shutdown hook
    ///
    /// Shutdown hook 会在应用关闭时按注册顺序执行
    pub fn shutdown_hook<F>(mut self, hook: F) -> Self
    where
        F: Fn() -> ContainerResult<()> + Send + Sync + 'static,
    {
        self.shutdown_hooks.push(Box::new(hook));
        self
    }

    /// 运行应用
    pub async fn run(self) -> ApplicationResult<RunningApplication> {
        // 初始化日志系统
        let logging_config = self.logging_config.clone().unwrap_or_else(LoggingConfig::from_env);
        logging_config.init()?;

        // 记录启动开始时间
        let start_time = std::time::Instant::now();

        // 显示 banner
        if self.show_banner {
            self.print_banner();
        }

        tracing::info!("Starting {} application", self.name);

        // 解析 active profiles
        // 优先级：代码设置 > 环境变量 APP_PROFILES_ACTIVE
        let mut active_profiles = self.profiles.clone();
        if active_profiles.is_empty() {
            // 尝试从环境变量读取
            if let Ok(profiles_str) = std::env::var(format!("{}PROFILES_ACTIVE", self.env_prefix)) {
                active_profiles = profiles_str.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }

        if !active_profiles.is_empty() {
            tracing::info!("Active profiles: {:?}", active_profiles);
        } else {
            tracing::info!("No active profiles set, using default configuration");
        }

        // 创建 ApplicationContext Builder
        let mut builder = ApplicationContext::builder();

        // 加载配置文件（按优先级：default -> profile specific -> environment）
        self.load_configurations(&mut builder, &active_profiles)?;

        // 添加环境变量配置源（优先级最高）
        builder = builder.add_property_source(Box::new(
            EnvironmentPropertySource::new(&self.env_prefix)
        ));
        tracing::debug!("Environment variable prefix: {}", self.env_prefix);

        // 设置 profiles
        builder = builder.set_active_profiles(active_profiles);

        // 构建 ApplicationContext
        let context = builder.build().await?;
        tracing::info!("ApplicationContext creating");

        // 设置应用名称（用于事件）
        context.set_app_name(self.name.clone()).await;

        // 执行自定义初始化器（在扫描组件之前）
        for initializer in &self.initializers {
            initializer(&context)?;
        }

        // 执行插件配置阶段
        tracing::info!("Configuring plugins");
        self.plugin_registry.configure_all(&context)?;

        // 自动扫描并绑定 ConfigurationProperties
        tracing::info!("Scanning for @ConfigurationProperties annotated beans");
        context.scan_configuration_properties().await?;

        // 自动扫描组件
        tracing::info!("Scanning for @Component annotated beans");
        context.scan_components().await?;

        // 自动扫描并注册EventListener
        tracing::info!("Scanning for EventListener implementations");
        context.scan_event_listeners().await?;

        // 验证依赖
        tracing::info!("Validating bean dependencies");
        context.validate_dependencies().await?;

        // 初始化所有非延迟加载的单例 Bean（包括刚扫描到的 Component）
        tracing::info!("Initializing non-lazy singleton beans");
        context.initialize().await?;
        tracing::info!("ApplicationContext initialized");

        // 计算启动耗时
        let elapsed = start_time.elapsed();
        let elapsed_ms = elapsed.as_millis();

        tracing::info!("Started {} in {}ms", self.name, elapsed_ms);

        // 发布 ApplicationStartedEvent
        let event = Arc::new(ApplicationStartedEvent::new(
            self.name.clone(),
            elapsed_ms,
        ));
        context.publish_event(event).await;

        // 注册在 ChimeraApplication 中配置的 shutdown hooks
        for hook in self.shutdown_hooks {
            context.register_shutdown_hook(hook).await;
        }

        // 执行插件启动阶段
        tracing::info!("Starting plugins");
        self.plugin_registry.startup_all(&context).await?;

        // 设置优雅停机信号处理（Ctrl+C）
        let context_for_signal = Arc::clone(&context);
        let plugin_registry_for_signal = self.plugin_registry;
        tokio::spawn(async move {
            match tokio::signal::ctrl_c().await {
                Ok(()) => {
                    tracing::info!("Received shutdown signal (Ctrl+C), initiating graceful shutdown");

                    // 先关闭插件
                    if let Err(e) = plugin_registry_for_signal.shutdown_all(&context_for_signal).await {
                        tracing::error!("Error during plugin shutdown: {}", e);
                    }

                    // 再关闭应用上下文
                    if let Err(e) = context_for_signal.shutdown().await {
                        tracing::error!("Error during context shutdown: {}", e);
                        std::process::exit(1);
                    }
                    std::process::exit(0);
                }
                Err(err) => {
                    tracing::error!("Unable to listen for shutdown signal: {}", err);
                }
            }
        });
        tracing::info!("Graceful shutdown hook registered (Ctrl+C to shutdown)");

        Ok(RunningApplication { context })
    }

    /// 加载配置文件
    ///
    /// 加载顺序（优先级从低到高）：
    /// 1. application.toml (default)
    /// 2. application-{profile}.toml (profile specific)
    ///
    /// 配置文件查找顺序（类似Spring Boot）：
    /// - 如果用户手动指定了配置文件路径，则使用指定的路径
    /// - 如果未指定，按以下顺序查找（找到第一个即停止）：
    ///   1. config/application.toml
    ///   2. application.toml
    ///
    /// 后加载的配置会覆盖先加载的配置
    fn load_configurations(
        &self,
        builder: &mut crate::container::ApplicationContextBuilder,
        active_profiles: &[String],
    ) -> ApplicationResult<()> {
        // 确定要使用的配置文件列表
        let config_files = if self.config_files.is_empty() {
            // 用户未指定配置文件，使用默认查找规则
            self.find_default_config_files()
        } else {
            // 用户已指定配置文件，直接使用
            self.config_files.clone()
        };

        // 1. 加载默认配置文件 (application.toml)
        for base_config in &config_files {
            self.try_load_config_file(builder, base_config, 0)?;
        }

        // 2. 加载 profile 特定配置文件
        // application-dev.toml, application-prod.toml, etc.
        for (index, profile) in active_profiles.iter().enumerate() {
            for base_config in &config_files {
                // 从 application.toml 推导出 application-dev.toml
                let profile_config = self.get_profile_config_path(base_config, profile);
                // 优先级递增：profile 配置优先级高于默认配置
                self.try_load_config_file(builder, &profile_config, 10 + index as i32)?;
            }
        }

        Ok(())
    }

    /// 查找默认配置文件
    ///
    /// 按照以下顺序查找，找到第一个存在的文件即返回：
    /// 1. config/application.toml
    /// 2. application.toml
    fn find_default_config_files(&self) -> Vec<String> {
        let candidates = vec![
            "config/application.toml",
            "application.toml",
        ];

        for candidate in candidates {
            if Path::new(candidate).exists() {
                tracing::debug!("Found configuration file: {}", candidate);
                return vec![candidate.to_string()];
            }
        }

        // 如果都不存在，返回默认值（config/application.toml）
        // 这样在日志中会显示找不到配置文件，但不会报错
        tracing::debug!("No configuration file found, using default path: config/application.toml");
        vec!["config/application.toml".to_string()]
    }

    /// 获取 profile 配置文件路径
    ///
    /// 例如：application.toml -> application-dev.toml
    fn get_profile_config_path(&self, base_path: &str, profile: &str) -> String {
        if let Some(dot_pos) = base_path.rfind('.') {
            let (name, ext) = base_path.split_at(dot_pos);
            format!("{}-{}{}", name, profile, ext)
        } else {
            format!("{}-{}", base_path, profile)
        }
    }

    /// 尝试加载配置文件
    fn try_load_config_file(
        &self,
        builder: &mut crate::container::ApplicationContextBuilder,
        config_file: &str,
        priority: i32,
    ) -> ApplicationResult<()> {
        if Path::new(config_file).exists() {
            match TomlPropertySource::from_file(config_file) {
                Ok(source) => {
                    tracing::info!("Loaded configuration from: {} (priority: {})", config_file, priority);
                    builder.add_property_source_mut(
                        Box::new(source.with_priority(priority))
                    );
                }
                Err(e) => {
                    tracing::warn!("Failed to load {}: {}", config_file, e);
                }
            }
        } else {
            tracing::debug!("Configuration file not found: {}", config_file);
        }
        Ok(())
    }

    /// 便捷方法：使用默认配置运行
    pub async fn run_with_defaults(name: impl Into<String>) -> ApplicationResult<RunningApplication> {
        Self::new(name).run().await
    }

    /// 便捷方法：运行应用并阻塞直到关闭（类似 Spring Boot）
    ///
    /// 这是一个便捷方法，等价于 `run().await?.wait_for_shutdown().await`
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use chimera_core::ChimeraApplication;
    ///
    /// #[tokio::main]
    /// async fn main() -> chimera_core::ApplicationResult<()> {
    ///     // 一行启动应用，自动阻塞直到收到关闭信号
    ///     ChimeraApplication::new("MyApp")
    ///         .env_prefix("APP_")
    ///         .run_until_shutdown()
    ///         .await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn run_until_shutdown(self) -> ApplicationResult<()> {
        self.run().await?.wait_for_shutdown().await
    }

    /// 打印 banner
    fn print_banner(&self) {
        println!();
        println!(r"   ____ _     _                           ");
        println!(r"  / ___| |__ (_)_ __ ___   ___ _ __ __ _ ");
        println!(r" | |   | '_ \| | '_ ` _ \ / _ \ '__/ _` |");
        println!(r" | |___| | | | | | | | | |  __/ | | (_| |");
        println!(r"  \____|_| |_|_|_| |_| |_|\___|_|  \__,_|");
        println!();
        println!("  :: Chimera Framework ::        (v{})", env!("CARGO_PKG_VERSION"));
        println!();
    }
}

impl Default for ChimeraApplication {
    fn default() -> Self {
        Self::new("ChimeraApplication")
    }
}

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

    /// 手动触发关闭
    pub fn shutdown(self) -> ApplicationResult<()> {
        self.context.shutdown().map_err(|e| e.into())
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
    /// 配置文件路径
    config_files: Vec<String>,

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
    ///
    /// 应用名称将从配置文件中的 chimera.app.name 读取，
    /// 如果配置文件未指定，则使用默认值 "application"
    pub fn new() -> Self {
        Self {
            config_files: Vec::new(), // 初始为空，将在 run 时根据规则查找
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
        use crate::constants::*;

        // 初始化日志系统
        let logging_config = self.logging_config.clone().unwrap_or_else(LoggingConfig::from_env);
        logging_config.init()?;

        // 记录启动开始时间
        let start_time = std::time::Instant::now();

        // 显示 banner
        if self.show_banner {
            self.print_banner();
        }

        // 首先创建一个临时的 builder 用于加载配置
        let mut temp_builder = ApplicationContext::builder();

        // 加载基础配置（不含 profile）以便读取框架配置
        // 这样我们可以从配置文件中读取 app name 和其他框架设置
        let config_dirs = vec!["config".to_string(), ".".to_string()];
        for dir in &config_dirs {
            let config_path = Path::new(dir).join("application.toml");
            if config_path.exists() {
                tracing::debug!("Loading base configuration from: {}", config_path.display());
                temp_builder = temp_builder.add_property_source(Box::new(
                    TomlPropertySource::from_file(&config_path)
                        .map_err(|e| crate::error::ApplicationError::ConfigLoadFailed(e.to_string()))?,
                ));
                break;
            }
        }

        // 添加环境变量配置源（优先级最高）
        temp_builder = temp_builder.add_property_source(Box::new(
            EnvironmentPropertySource::new()
        ));

        // 构建临时 context 仅用于读取配置
        let temp_context = temp_builder.build()?;

        // 从配置中读取 app name（优先级：配置文件 > 默认值）
        let app_name = temp_context
            .environment()
            .get_string(CONFIG_APP_NAME)
            .unwrap_or_else(|| DEFAULT_APP_NAME.to_string());

        // 从配置中读取是否使用异步事件（默认为 false）
        let async_events = temp_context
            .environment()
            .get_bool(CONFIG_EVENTS_ASYNC)
            .unwrap_or(false);

        tracing::info!("Starting {} application", app_name);

        // 解析 active profiles
        // 优先级：环境变量 > 配置文件 > 代码设置
        let mut active_profiles = Vec::new();

        // 1. 先从环境变量读取（最高优先级）
        if let Ok(profiles_str) = std::env::var(ENV_PROFILES_ACTIVE) {
            active_profiles = profiles_str.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            tracing::debug!("Profiles from environment variable: {:?}", active_profiles);
        }

        // 2. 如果环境变量没有，尝试从配置文件读取
        if active_profiles.is_empty() {
            if let Some(profiles_from_config) = temp_context.environment().get_string_array(CONFIG_PROFILES_ACTIVE) {
                active_profiles = profiles_from_config;
                tracing::debug!("Profiles from configuration file: {:?}", active_profiles);
            }
        }

        // 3. 如果配置文件也没有，使用代码中设置的
        if active_profiles.is_empty() && !self.profiles.is_empty() {
            active_profiles = self.profiles.clone();
            tracing::debug!("Profiles from code: {:?}", active_profiles);
        }

        if !active_profiles.is_empty() {
            tracing::info!("Active profiles: {:?}", active_profiles);
        } else {
            tracing::info!("No active profiles set, using default configuration");
        }

        // 创建正式的 ApplicationContext Builder，使用配置中读取的设置
        let mut builder = ApplicationContext::builder()
            .async_events(async_events);

        // 加载配置文件（按优先级：default -> profile specific -> environment）
        self.load_configurations(&mut builder, &active_profiles)?;

        // 添加环境变量配置源（优先级最高）
        builder = builder.add_property_source(Box::new(
            EnvironmentPropertySource::new()
        ));
        tracing::debug!("Environment variable prefix: {}", ENV_PREFIX);

        // 设置 profiles
        builder = builder.set_active_profiles(active_profiles);

        // 构建 ApplicationContext
        let context = builder.build()?;
        tracing::info!("ApplicationContext creating");

        // 设置应用名称（使用从配置读取的名称）
        context.set_app_name(app_name.clone());

        // 执行自定义初始化器（在扫描组件之前）
        for initializer in &self.initializers {
            initializer(&context)?;
        }

        // 执行插件配置阶段
        tracing::info!("Configuring plugins");
        self.plugin_registry.configure_all(&context)?;

        // 自动扫描并绑定 ConfigurationProperties
        tracing::info!("Scanning for @ConfigurationProperties annotated beans");
        context.scan_configuration_properties()?;

        // 自动扫描组件
        tracing::info!("Scanning for @Component annotated beans");
        context.scan_components()?;

        // 自动扫描并注册 @Bean 方法（需要在组件扫描之后，因为配置类本身是 Component）
        tracing::info!("Scanning for @Bean annotated methods");
        context.scan_bean_methods()?;

        // 自动扫描并注册 BeanFactoryPostProcessor（在 Bean 实例化之前）
        tracing::info!("Scanning for @BeanFactoryPostProcessor annotated processors");
        context.scan_bean_factory_post_processors();

        // 调用所有 BeanFactoryPostProcessor（在 Bean 定义加载后、Bean 实例化之前）
        tracing::info!("Invoking BeanFactoryPostProcessors");
        context.invoke_bean_factory_post_processors()?;

        // 自动扫描并注册 BeanPostProcessor
        tracing::info!("Scanning for @BeanPostProcessor annotated processors");
        context.scan_bean_post_processors();

        // 自动扫描并注册EventListener
        tracing::info!("Scanning for EventListener implementations");
        context.scan_event_listeners()?;

        // 验证依赖
        tracing::info!("Validating bean dependencies");
        context.validate_dependencies()?;

        // 初始化所有非延迟加载的单例 Bean
        // 使用拓扑排序自动确定正确的初始化顺序（依赖的 bean 会先于依赖它的 bean 初始化）
        tracing::info!("Initializing non-lazy singleton beans");
        context.initialize()?;
        tracing::info!("ApplicationContext initialized");

        // 注册在 ChimeraApplication 中配置的 shutdown hooks
        for hook in self.shutdown_hooks {
            context.register_shutdown_hook(hook);
        }

        // 执行插件启动阶段
        tracing::info!("Starting plugins");
        self.plugin_registry.startup_all(&context).await?;

        // 计算启动耗时（包含插件启动时间）
        let elapsed = start_time.elapsed();
        let elapsed_ms = elapsed.as_millis();

        tracing::info!("Started {} in {} ms", app_name, elapsed_ms);

        // 发布 ApplicationStartedEvent
        let event = Arc::new(ApplicationStartedEvent::new(
            app_name.clone(),
            elapsed_ms,
        ));
        context.publish_event(event);

        // 检查是否有需要保持应用运行的插件
        let needs_keep_alive = self.plugin_registry.has_keep_alive_plugin();

        if needs_keep_alive {
            tracing::info!("Application will keep running (has keep-alive plugins)");

            // 设置优雅停机信号处理（Ctrl+C）
            let context_for_signal = Arc::clone(&context);
            let plugin_registry_for_signal = self.plugin_registry;
            tokio::spawn(async move {
                match tokio::signal::ctrl_c().await {
                    Ok(()) => {
                        tracing::info!(
                            "Received shutdown signal (Ctrl+C), initiating graceful shutdown"
                        );

                        // 先关闭插件
                        if let Err(e) = plugin_registry_for_signal
                            .shutdown_all(&context_for_signal)
                            .await
                        {
                            tracing::error!("Error during plugin shutdown: {}", e);
                        }

                        // 再关闭应用上下文
                        if let Err(e) = context_for_signal.shutdown() {
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

            // 阻塞直到收到关闭信号
            let () = std::future::pending().await;
        } else {
            tracing::info!("Application started successfully (no keep-alive plugins, will exit after run)");
        }

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
        builder: &mut crate::context::ApplicationContextBuilder,
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
        builder: &mut crate::context::ApplicationContextBuilder,
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
        Self::new()
    }
}

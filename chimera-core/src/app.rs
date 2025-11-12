use crate::{ApplicationContext, ApplicationResult};
use crate::config::{TomlPropertySource, EnvironmentPropertySource};
use crate::event::ApplicationStartedEvent;
use crate::logging::LoggingConfig;
use std::sync::Arc;
use std::path::Path;

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
}

impl ChimeraApplication {
    /// 创建新的应用
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            config_files: vec!["application.toml".to_string()],
            env_prefix: "APP_".to_string(),
            profiles: Vec::new(),
            show_banner: true,
            logging_config: None,
            initializers: Vec::new(),
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

    /// 运行应用
    pub async fn run(self) -> ApplicationResult<Arc<ApplicationContext>> {
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

        // 执行自定义初始化器（在扫描组件之前）
        for initializer in &self.initializers {
            initializer(&context)?;
        }

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

        Ok(context)
    }

    /// 加载配置文件
    ///
    /// 加载顺序（优先级从低到高）：
    /// 1. application.toml (default)
    /// 2. application-{profile}.toml (profile specific)
    ///
    /// 后加载的配置会覆盖先加载的配置
    fn load_configurations(
        &self,
        builder: &mut crate::container::ApplicationContextBuilder,
        active_profiles: &[String],
    ) -> ApplicationResult<()> {
        // 1. 加载默认配置文件 (application.toml)
        for base_config in &self.config_files {
            self.try_load_config_file(builder, base_config, 0)?;
        }

        // 2. 加载 profile 特定配置文件
        // application-dev.toml, application-prod.toml, etc.
        for (index, profile) in active_profiles.iter().enumerate() {
            for base_config in &self.config_files {
                // 从 application.toml 推导出 application-dev.toml
                let profile_config = self.get_profile_config_path(base_config, profile);
                // 优先级递增：profile 配置优先级高于默认配置
                self.try_load_config_file(builder, &profile_config, 10 + index as i32)?;
            }
        }

        Ok(())
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
    pub async fn run_with_defaults(name: impl Into<String>) -> ApplicationResult<Arc<ApplicationContext>> {
        Self::new(name).run().await
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

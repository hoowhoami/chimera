use chimera_core::prelude::*;
use chimera_core::async_trait;
use chimera_macros::{Component, ConfigurationProperties};
use std::sync::Arc;

// ==================== 事件监听器定义 ====================

/// 应用启动事件监听器
#[derive(Component, Clone, Debug)]
#[bean("startupListener")]
#[event_listener]
struct StartupEventListener {
    #[autowired]
    app_config: Arc<AppConfig>,
}

#[async_trait::async_trait]
impl EventListener for StartupEventListener {
    async fn on_event(&self, event: Arc<dyn Event>) {
        if let Some(started_event) = event.as_any().downcast_ref::<ApplicationStartedEvent>() {
            println!("\n📢 [StartupEventListener] Application started event received!");
            println!("   Application: {}", started_event.app_name);
            println!("   Startup time: {}ms", started_event.startup_time_ms);
            println!("   App name: {}", self.app_config.name);
        }
    }

    fn listener_name(&self) -> &str {
        "StartupEventListener"
    }

    fn supports_event(&self, event_name: &str) -> bool {
        event_name == "ApplicationStartedEvent"
    }
}

/// 自定义事件监听器 - 监听所有事件
#[derive(Component, Clone, Debug)]
#[bean("loggingListener")]
#[event_listener]
struct LoggingEventListener;

#[async_trait::async_trait]
impl EventListener for LoggingEventListener {
    async fn on_event(&self, event: Arc<dyn Event>) {
        println!("📝 [LoggingListener] Event received: {}", event.event_name());
    }

    fn listener_name(&self) -> &str {
        "LoggingEventListener"
    }
}

// ==================== 配置定义 ====================

/// 应用配置 - 使用 @ConfigurationProperties 自动绑定
#[derive(ConfigurationProperties, Debug, Clone)]
#[prefix("app")]
struct AppConfig {
    name: String,
    version: String,
}

/// 数据库配置 - 使用 @ConfigurationProperties 自动绑定
#[derive(ConfigurationProperties, Debug, Clone)]
#[prefix("database")]
struct DatabaseConfig {
    host: String,
    port: i32,

    #[config("max-connections")]
    max_connections: i32,
}

/// 服务器配置 - 使用 @ConfigurationProperties 自动绑定
#[derive(ConfigurationProperties, Debug, Clone)]
#[prefix("server")]
struct ServerConfig {
    host: String,
    port: i32,
    workers: i32,
}

#[derive(Component, Clone, Debug)]
#[lazy]
struct CommonService {
    
}

impl CommonService {
    fn print(&self) -> String {
        String::from("CommonService init...")
    }
}


pub type Result<T> = std::result::Result<T, ApplicationError>;

// ==================== 业务服务 ====================

/// 数据库服务 - 自动注入配置
#[derive(Component, Debug, Clone)]
#[bean("databaseService")]
struct DatabaseService {
    #[autowired]
    config: Arc<DatabaseConfig>,
}

impl DatabaseService {
    fn connect(&self) -> Result<()> {
        println!("📊 Connecting to database: {}:{}", self.config.host, self.config.port);
        println!("   Max connections: {}", self.config.max_connections);
        Ok(())
    }

    fn query(&self, sql: &str) -> Result<String> {
        Ok(format!("Query result for: {}", sql))
    }
}

/// 服务器服务 - 自动注入配置和依赖
#[derive(Component, Debug)]
#[bean("serverService")]
#[init]       // 使用默认的 init 方法
#[destroy]    // 使用默认的 destroy 方法
struct ServerService {
    #[autowired]
    config: Arc<ServerConfig>,

    #[autowired]
    db: Arc<DatabaseService>,

    #[autowired]
    app_config: Arc<AppConfig>,
}

impl ServerService {
    // 初始化回调（类似 Spring 的 @PostConstruct）
    fn init(&mut self) -> ContainerResult<()> {
        println!("🎉 ServerService initialized!");
        println!("   Verifying configuration...");
        println!("   Server will bind to: {}:{}", self.config.host, self.config.port);
        println!("   Database endpoint: {}:{}", self.db.config.host, self.db.config.port);
        println!("   ✅ Initialization complete!");
        Ok(())
    }

    // 销毁回调（类似 Spring 的 @PreDestroy）
    fn destroy(&mut self) -> ContainerResult<()> {
        println!("👋 ServerService shutting down...");
        println!("   Cleaning up resources...");
        println!("   Closing connections...");
        println!("   ✅ Cleanup complete!");
        Ok(())
    }

    fn start(&self) -> Result<()> {
        println!("\n╔════════════════════════════════════════════════════╗");
        println!("║  {} v{}", self.app_config.name, self.app_config.version);
        println!("╚════════════════════════════════════════════════════╝\n");

        println!("🚀 Starting server...");
        println!("   Host: {}", self.config.host);
        println!("   Port: {}", self.config.port);
        println!("   Workers: {}", self.config.workers);

        // 连接数据库
        self.db.connect()?;

        println!("\n✅ Server is running!");
        Ok(())
    }

    fn handle_request(&self, path: &str) -> Result<()> {
        println!("\n🔧 Handling request: {}", path);
        let result = self.db.query("SELECT * FROM users")?;
        println!("   Response: {}", result);
        Ok(())
    }
}

// ==================== 主程序 ====================


#[tokio::main]
async fn main() -> ApplicationResult<()> {
    println!("\n╔════════════════════════════════════════════════════╗");
    println!("║     Chimera Framework - Complete Demo            ║");
    println!("╚════════════════════════════════════════════════════╝\n");

    // 查找配置文件
    let config_paths = vec![
        "examples/app-demo/application.toml",
        "application.toml",
    ];

    let mut config_file = "application.toml";
    for path in &config_paths {
        if std::path::Path::new(path).exists() {
            config_file = path;
            break;
        }
    }

    // ✅ 使用 ChimeraApplication.run() 启动应用
    // 自动完成：
    //   1. 加载配置文件 (application.toml)
    //   2. 扫描并绑定 @ConfigurationProperties
    //   3. 扫描并注册 @Component
    //   4. 自动依赖注入
    //   5. 并发初始化所有 bean
    //   6. 自动扫描并注册EventListener
    let context = ChimeraApplication::new("ChimeraDemo")
        .config_file(config_file)
        .env_prefix("APP_")
        .run().await?;

    println!("\n╔════════════════════════════════════════════════════╗");
    println!("║              Application Started                  ║");
    println!("╚════════════════════════════════════════════════════╝");

    // 在一个作用域中使用beans，确保在shutdown前释放所有引用
    {
        // 获取并使用服务
        let server = context.get_bean_by_type::<ServerService>().await?;
        server.start()?;

        // 模拟处理请求
        server.handle_request("/api/users")?;

        // 显示所有配置
        println!("\n╔════════════════════════════════════════════════════╗");
        println!("║           Configuration Summary                   ║");
        println!("╚════════════════════════════════════════════════════╝\n");

        let app_config = context.get_bean_by_type::<AppConfig>().await?;
        let db_config = context.get_bean_by_type::<DatabaseConfig>().await?;
        let server_config = context.get_bean_by_type::<ServerConfig>().await?;

        println!("📦 Application:");
        println!("   Name: {}", app_config.name);
        println!("   Version: {}", app_config.version);

        println!("\n🗄️  Database:");
        println!("   Host: {}", db_config.host);
        println!("   Port: {}", db_config.port);
        println!("   Max Connections: {}", db_config.max_connections);

        println!("\n🖥️  Server:");
        println!("   Host: {}", server_config.host);
        println!("   Port: {}", server_config.port);
        println!("   Workers: {}", server_config.workers);

        let common_service = context.get_bean_by_type::<CommonService>().await?;
        println!("\nCommonService print: {}", common_service.print());
    } // 所有bean引用在这里被释放

    // ==================== 演示事件系统 ====================
    println!("\n╔════════════════════════════════════════════════════╗");
    println!("║              Event System Demo                    ║");
    println!("╚════════════════════════════════════════════════════╝\n");

    // 发布自定义事件
    println!("📤 Publishing custom events...\n");

    let custom_event1 = Arc::new(CustomEvent::new(
        "UserLoginEvent".to_string(),
        Arc::new("user123".to_string()),
    ));
    context.publish_event(custom_event1).await;

    let custom_event2 = Arc::new(CustomEvent::new(
        "OrderCreatedEvent".to_string(),
        Arc::new(12345_i32),
    ));
    context.publish_event(custom_event2).await;

    println!();

    println!("\n╔════════════════════════════════════════════════════╗");
    println!("║                Key Features                       ║");
    println!("╚════════════════════════════════════════════════════╝\n");

    println!("✅ @ConfigurationProperties - 自动批量绑定配置");
    println!("✅ @Component - 自动组件扫描和注册");
    println!("✅ @autowired - 自动依赖注入");
    println!("✅ 类型安全的配置管理");
    println!("✅ 环境变量覆盖 (APP_* 前缀)");
    println!("✅ Spring Boot 风格的开发体验");
    println!("✅ 异步初始化 + 并发bean创建");
    println!("✅ Event/Publisher/Listener - 事件驱动架构");

    println!("\n💡 Try these commands:");
    println!("   APP_SERVER_PORT=9000 cargo run -p app-demo");
    println!("   APP_DATABASE_HOST=prod-db cargo run -p app-demo");

    println!();

    // 演示生命周期回调：shutdown 时会调用 @PreDestroy
    println!("\n╔════════════════════════════════════════════════════╗");
    println!("║           Shutting Down Application              ║");
    println!("╚════════════════════════════════════════════════════╝\n");

    context.shutdown().await?;

    println!("\n✅ Application shutdown complete!");

    Ok(())
}

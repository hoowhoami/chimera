use chimera_core::prelude::*;
use chimera_core_macros::{component, Component, ConfigurationProperties};
use std::sync::Arc;
use std::time::SystemTime;

// ==================== 事件定义 ====================

#[derive(Debug, Clone)]
pub struct UserRegisteredEvent {
    pub user_id: String,
    pub username: String,
    pub timestamp: SystemTime,
}

impl UserRegisteredEvent {
    pub fn new(user_id: String, username: String) -> Self {
        Self {
            user_id,
            username,
            timestamp: SystemTime::now(),
        }
    }
}

impl Event for UserRegisteredEvent {
    fn event_name(&self) -> &str {
        "UserRegisteredEvent"
    }

    fn timestamp(&self) -> SystemTime {
        self.timestamp
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// ==================== 配置 ====================

#[derive(ConfigurationProperties, Debug, Clone)]
#[prefix("app")]
struct AppConfig {
    name: String,
    version: String,
    environment: String,
}

#[derive(ConfigurationProperties, Debug, Clone)]
#[prefix("database")]
struct DatabaseConfig {
    url: String,
    pool_size: i32,
    timeout_ms: i32,
}

#[derive(ConfigurationProperties, Debug, Clone)]
#[prefix("redis")]
struct RedisConfig {
    host: String,
    port: i32,
    #[config("max-connections")]
    max_connections: i32,
}

// ==================== 服务层 ====================

#[derive(Component)]
#[component("systemService")]
struct SystemService {
    #[autowired]
    context: Arc<ApplicationContext>,

    #[autowired]
    environment: Arc<Environment>,

    #[autowired]
    event_publisher: Arc<ApplicationEventPublisher>,

    #[autowired]
    bean_factory: Arc<DefaultListableBeanFactory>,

    test: String,
}

#[component]
impl SystemService {
    async fn demonstrate_core_components(&self) -> ApplicationResult<()> {
        println!("System Service - Core Components Injection Demo:");

        // 使用注入的 Environment
        println!(
            "  Environment active profiles: {:?}",
            self.environment.get_active_profiles()
        );

        if let Some(app_name) = self.environment.get_string("app.name") {
            println!("  App name from injected environment: {}", app_name);
        }

        // 使用注入的 ApplicationContext
        let bean_names = self.context.get_bean_names();
        println!("  Total beans from injected context: {}", bean_names.len());

        // 使用注入的 EventPublisher 发布事件
        let custom_event = Arc::new(SystemHealthCheckEvent::new(
            "All core components injected successfully".to_string(),
        ));
        self.event_publisher.publish_event(custom_event);
        println!("  Published event using injected EventPublisher");

        // 使用注入的 BeanFactory
        let definitions = self.bean_factory.get_bean_definitions();
        println!("  Total bean definitions from injected BeanFactory: {}", definitions.len());

        println!("  ALL core components (ApplicationContext, Environment, EventPublisher) successfully injected!");

        Ok(())
    }

    async fn demonstrate_context_usage(&self) -> ApplicationResult<()> {
        println!("🔍 Advanced Context Usage Demo:");

        // 通过注入的 ApplicationContext 动态获取其他 bean
        if let Ok(app_config) = self.context.get_bean_by_type::<AppConfig>() {
            println!(
                "  Retrieved AppConfig via injected context: {} v{}",
                app_config.name, app_config.version
            );
        }

        // 检查 bean 是否存在
        println!(
            "  Database service exists: {}",
            self.context.contains_bean("databaseService")
        );
        println!(
            "  Cache service exists: {}",
            self.context.contains_bean("cacheService")
        );

        Ok(())
    }
}

// 新增系统健康检查事件
#[derive(Debug, Clone)]
pub struct SystemHealthCheckEvent {
    pub message: String,
    pub timestamp: SystemTime,
}

impl SystemHealthCheckEvent {
    pub fn new(message: String) -> Self {
        Self {
            message,
            timestamp: SystemTime::now(),
        }
    }
}

impl Event for SystemHealthCheckEvent {
    fn event_name(&self) -> &str {
        "SystemHealthCheckEvent"
    }

    fn timestamp(&self) -> SystemTime {
        self.timestamp
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Component, Debug, Clone)]
#[component("cacheService")]
struct CacheService {
    #[autowired]
    redis_config: Arc<RedisConfig>,
}

#[component]
impl CacheService {
    fn set(&self, key: &str, value: &str) -> ApplicationResult<()> {
        println!(
            "Cache SET {}: {} (Redis: {}:{})",
            key, value, self.redis_config.host, self.redis_config.port
        );
        Ok(())
    }

    fn get(&self, key: &str) -> ApplicationResult<Option<String>> {
        println!(
            "Cache GET {} (connections: {})",
            key, self.redis_config.max_connections
        );
        Ok(Some(format!("cached_value_{}", key)))
    }
}

#[derive(Component, Debug)]
#[component("databaseService")]
#[init]
#[destroy]
struct DatabaseService {
    #[autowired]
    config: Arc<DatabaseConfig>,
}

#[component]
impl DatabaseService {
    fn init(&mut self) -> ContainerResult<()> {
        println!("Database connecting to: {}", self.config.url);
        Ok(())
    }

    fn destroy(&mut self) -> ContainerResult<()> {
        println!(
            "Database closing connections (pool size: {})",
            self.config.pool_size
        );
        Ok(())
    }

    fn save_user(&self, user_id: &str, username: &str) -> ApplicationResult<()> {
        println!(
            "Database saving user: {} ({}) [timeout: {}ms]",
            username, user_id, self.config.timeout_ms
        );
        Ok(())
    }
}

#[derive(Component, Debug, Clone)]
#[lazy]
#[component("userService")]
struct UserService {
    #[autowired]
    database: Arc<DatabaseService>,

    #[autowired]
    cache: Arc<CacheService>,

    #[autowired]
    app_config: Arc<AppConfig>,
}

#[component]
impl UserService {
    fn register_user(&self, username: &str) -> ApplicationResult<String> {
        let user_id = format!("user_{}", rand::random::<u32>());

        // 业务逻辑演示
        self.database.save_user(&user_id, username)?;
        self.cache.set(&format!("user:{}", user_id), username)?;

        println!(
            "User registered: {} in {}",
            username, self.app_config.environment
        );
        Ok(user_id)
    }

    fn get_user(&self, user_id: &str) -> ApplicationResult<Option<String>> {
        // 先尝试从缓存获取
        if let Ok(Some(cached)) = self.cache.get(&format!("user:{}", user_id)) {
            println!("User found in cache: {}", cached);
            return Ok(Some(cached));
        }

        println!("User not in cache, querying database...");
        Ok(Some(format!("user_from_db_{}", user_id)))
    }
}

// ==================== 可选依赖演示 ====================

// 一个可选的服务，可能存在也可能不存在
// #[derive(Component, Debug, Clone)]
// #[component("metricsService")]
struct MetricsService {}

impl MetricsService {
    fn track(&self, metric: &str, value: i64) {
        println!("📊 Metrics: {} = {}", metric, value);
    }
}

// 使用可选依赖的服务
#[derive(Component, Clone)]
#[component("orderService")]
struct OrderService {
    #[autowired]
    database: Arc<DatabaseService>,

    // 可选依赖：如果 MetricsService 存在就使用，不存在也不影响服务运行
    #[autowired]
    metrics: Option<Arc<MetricsService>>,
}

#[component]
impl OrderService {
    fn create_order(&self, order_id: &str, amount: i64) -> ApplicationResult<()> {
        println!("Creating order: {} (amount: {})", order_id, amount);

        // 保存到数据库
        self.database.save_user(order_id, "order_data")?;

        // 如果有 metrics 服务，就记录指标
        if let Some(metrics) = &self.metrics {
            metrics.track("order.created", 1);
            metrics.track("order.amount", amount);
            println!("   Metrics tracked");
        } else {
            println!("   Metrics service not available (optional)");
        }

        Ok(())
    }
}

// 测试不存在的可选依赖
#[derive(Component, Debug, Clone)]
#[component("paymentService")]
struct PaymentService {
    // 这个服务不存在，用于测试可选依赖为 None 的情况
    #[autowired("nonExistentService")]
    optional_service: Option<Arc<CacheService>>,
}

#[component]
impl PaymentService {
    fn process_payment(&self, amount: i64) -> ApplicationResult<()> {
        println!("💳 Processing payment: {}", amount);

        if let Some(service) = &self.optional_service {
            println!("   Using optional service: {:?}", service);
        } else {
            println!("   Optional service 'nonExistentService' not found (as expected)");
        }

        Ok(())
    }
}

// ==================== 事件监听器 ====================

#[derive(Component, Clone, Debug)]
#[event_listener]
struct NotificationService {
    #[autowired]
    app_config: Arc<AppConfig>,
}

#[component]
impl EventListener for NotificationService {
    fn on_event(&self, event: Arc<dyn Event>) {
        match event.event_name() {
            "ApplicationStartedEvent" => {
                println!(
                    "🎉 {} v{} started successfully",
                    self.app_config.name, self.app_config.version
                );
            }
            "UserRegisteredEvent" => {
                if let Some(user_event) = event.as_any().downcast_ref::<UserRegisteredEvent>() {
                    println!("📧 Welcome email sent to user: {}", user_event.username);
                }
            }
            "SystemHealthCheckEvent" => {
                if let Some(health_event) = event.as_any().downcast_ref::<SystemHealthCheckEvent>()
                {
                    println!("💚 System Health: {}", health_event.message);
                }
            }
            "ApplicationShutdownEvent" => {
                println!("👋 Application shutting down gracefully");
            }
            _ => {}
        }
    }

    fn listener_name(&self) -> &str {
        "NotificationService"
    }
}

#[derive(Component, Clone, Debug)]
struct AuditService;

#[component]
impl TypedEventListener<UserRegisteredEvent> for AuditService {
    fn on_event(&self, event: &UserRegisteredEvent) {
        println!(
            "📋 Audit log: User {} ({}) registered at {:?}",
            event.username, event.user_id, event.timestamp
        );
    }

    fn listener_name(&self) -> &str {
        "AuditService"
    }
}

// ==================== 主程序 ====================

pub mod rand {
    pub fn random<T>() -> T
    where
        T: From<u8>,
    {
        // 简单的伪随机数生成器用于演示
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::time::{SystemTime, UNIX_EPOCH};

        let mut hasher = DefaultHasher::new();
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .hash(&mut hasher);
        T::from((hasher.finish() % 256) as u8)
    }
}

#[tokio::main]
async fn main() -> ApplicationResult<()> {
    println!("Chimera Framework - Comprehensive Demo\n");

    // 配置文件会自动从以下位置查找（按优先级）：
    // 1. config/application.toml
    // 2. application.toml
    // 支持 profile 特定配置：config/application-dev.toml, config/application-prod.toml 等

    // 启动应用
    let context = ChimeraApplication::new()
        .shutdown_hook(|| {
            println!("Cleaning up resources...");
            Ok(())
        })
        .shutdown_hook(|| {
            println!("Closing connections...");
            Ok(())
        })
        .run()
        .await?;

    // 注册类型化事件监听器
    {
        let audit_service = context.get_bean_by_type::<AuditService>()?;
        let adapter = TypedEventListenerAdapter::new(audit_service);
        context.register_listener(Arc::new(adapter));
    }

    println!("Application initialized\n");

    // 使用作用域确保引用在shutdown前释放
    {
        // 演示配置注入
        let app_config = context.get_bean_by_type::<AppConfig>()?;
        let db_config = context.get_bean_by_type::<DatabaseConfig>()?;
        let redis_config = context.get_bean_by_type::<RedisConfig>()?;

        println!("📋 Configuration Summary:");
        println!(
            "  App: {} v{} ({})",
            app_config.name, app_config.version, app_config.environment
        );
        println!(
            "  Database: {} (pool: {})",
            db_config.url, db_config.pool_size
        );
        println!(
            "  Redis: {}:{} (max: {})\n",
            redis_config.host, redis_config.port, redis_config.max_connections
        );

        // 演示核心组件注入
        println!("🧩 Core Components Injection Demo:");
        let system_service = context.get_bean_by_type::<SystemService>()?;
        system_service.demonstrate_core_components().await?;
        system_service.demonstrate_context_usage().await?;
        println!();

        // 演示业务逻辑
        println!("🔄 Business Logic Demo:");
        let user_service = context.get_bean_by_type::<UserService>()?;

        // 注册用户（触发事件）
        let user_id = user_service.register_user("alice")?;

        // 发布用户注册事件
        let event = Arc::new(UserRegisteredEvent::new(
            user_id.clone(),
            "alice".to_string(),
        ));
        context.publish_event(event);

        // 查询用户
        user_service.get_user(&user_id)?;

        println!();

        // 演示可选依赖
        println!("🔀 Optional Dependency Demo:");
        let order_service = context.get_bean_by_type::<OrderService>()?;
        order_service.create_order("ORDER-001", 9999)?;

        // 测试不存在的可选依赖
        let payment_service = context.get_bean_by_type::<PaymentService>()?;
        payment_service.process_payment(9999)?;
        println!();

        // 等待异步事件处理完成
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

        println!();
    } // 释放所有bean引用

    context.shutdown()?;

    println!("Demo completed successfully");
    println!("Framework features demonstrated:");
    println!("  • @ConfigurationProperties - Type-safe configuration");
    println!("  • @Component & @autowired - Dependency injection");
    println!("  • @autowired(\"beanName\") - Named bean injection");
    println!("  • Option<Arc<T>> - Optional dependencies");
    println!("  • @init & @destroy - Lifecycle management");
    println!("  • @lazy - Lazy initialization");
    println!("  • Event system - Typed & untyped listeners");
    println!("  • Core components injection - ApplicationContext, Environment, EventPublisher via @autowired");
    println!("  • Dynamic bean retrieval - Get beans by name and type at runtime");
    println!("  • Shutdown hooks - Graceful shutdown");

    Ok(())
}

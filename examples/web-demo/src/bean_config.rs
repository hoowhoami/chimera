//! Bean Configuration 示例
//!
//! 展示如何使用 Configuration + #[bean] 方法宏来定义和注册 beans
//! 类似 Spring Boot 的 @Configuration + @Bean 注解

use chimera_core::prelude::*;
use chimera_core_macros::{bean, configuration, destroy, init, lazy, scope, Configuration};
use std::sync::Arc;
use crate::config::AppConfig;
// ==================== 通过 Bean 方法创建的服务 ====================

/// 一个简单的邮件服务
#[derive(Debug, Clone)]
pub struct EmailService {
    smtp_host: String,
    smtp_port: u16,
}

impl EmailService {
    pub fn new(smtp_host: String, smtp_port: u16) -> Self {
        Self { smtp_host, smtp_port }
    }

    pub fn send_email(&self, to: &str, subject: &str, body: &str) {
        tracing::info!(
            "📧 Sending email to {} via {}:{} - Subject: {}",
            to,
            self.smtp_host,
            self.smtp_port,
            subject
        );
    }
}

/// 一个 SMS 短信服务
#[derive(Debug, Clone)]
pub struct SmsService {
    api_key: String,
    api_endpoint: String,
}

impl SmsService {
    pub fn new(api_key: String, api_endpoint: String) -> Self {
        Self { api_key, api_endpoint }
    }

    pub fn send_sms(&self, phone: &str, message: &str) {
        tracing::info!(
            "📱 Sending SMS to {} via {} - Message: {}",
            phone,
            self.api_endpoint,
            message
        );
    }
}

/// 通知服务，依赖于 EmailService 和 SmsService
#[derive(Debug, Clone)]
pub struct NotificationService {
    email: Arc<EmailService>,
    sms: Arc<SmsService>,
}

impl NotificationService {
    pub fn new(email: Arc<EmailService>, sms: Arc<SmsService>) -> Self {
        Self { email, sms }
    }

    pub fn notify(&self, user: &str, message: &str) {
        tracing::info!("🔔 Notifying user: {}", user);
        self.email.send_email(user, "Notification", message);
        self.sms.send_sms(user, message);
    }
}

/// 数据库连接池服务 - 展示生命周期管理
#[derive(Debug)]
pub struct DatabaseConnectionPool {
    pool_size: u32,
    active_connections: u32,
}

impl DatabaseConnectionPool {
    pub fn new(pool_size: u32) -> Self {
        Self {
            pool_size,
            active_connections: 0,
        }
    }

    /// 初始化方法 - 打开连接池
    pub fn init(&mut self) -> ContainerResult<()> {
        tracing::info!("🔌 Initializing database connection pool with size {}", self.pool_size);
        self.active_connections = self.pool_size;
        tracing::info!("✅ Connection pool initialized, {} connections active", self.active_connections);
        Ok(())
    }

    /// 销毁方法 - 关闭连接池
    pub fn destroy(&mut self) -> ContainerResult<()> {
        tracing::info!("🔌 Closing database connection pool");
        tracing::info!("📊 Active connections: {}", self.active_connections);
        self.active_connections = 0;
        tracing::info!("✅ Connection pool closed successfully");
        Ok(())
    }
}

/// 缓存服务 - 展示自定义生命周期方法名
#[derive(Debug)]
pub struct CacheManager {
    cache_name: String,
    entries: u32,
}

impl CacheManager {
    pub fn new(cache_name: String) -> Self {
        Self {
            cache_name,
            entries: 0,
        }
    }

    /// 自定义初始化方法
    pub fn startup(&mut self) -> ContainerResult<()> {
        tracing::info!("💾 Starting cache manager: {}", self.cache_name);
        self.entries = 100; // 预加载缓存
        tracing::info!("✅ Cache manager started with {} entries", self.entries);
        Ok(())
    }

    /// 自定义销毁方法
    pub fn cleanup(&mut self) -> ContainerResult<()> {
        tracing::info!("💾 Cleaning up cache manager: {}", self.cache_name);
        tracing::info!("📊 Total entries: {}", self.entries);
        self.entries = 0;
        tracing::info!("✅ Cache manager cleaned up");
        Ok(())
    }
}

// ==================== Configuration 类 ====================

/// Bean 配置类
///
/// 使用 #[derive(Configuration)] 标记配置类
/// 配置类本身也是一个 Component，会被自动注册到容器
///
/// 类似 Spring Boot 的 @Configuration 注解
#[derive(Configuration)]
pub struct BeanConfig {
    /// 可以注入 Environment 来获取配置
    #[autowired]
    environment: Arc<Environment>,
    #[autowired]
    context: Arc<ApplicationContext>,
}

#[configuration]
impl BeanConfig {
    /// 创建 EmailService bean
    ///
    /// #[bean] - 使用方法名作为 bean 名称（emailService）
    /// 方法直接返回类型 T，框架会自动包装成 Ok(T)
    ///
    /// 类似 Spring 的 @Bean 注解
    #[bean]
    pub fn email_service(&self) -> EmailService {
        let smtp_host = self.environment
            .get_string("email.smtp.host")
            .unwrap_or_else(|| "localhost".to_string());

        let smtp_port = self.environment
            .get_i64("email.smtp.port")
            .unwrap_or(25) as u16;

        tracing::info!(
            "📦 Creating EmailService bean with host={}, port={}",
            smtp_host,
            smtp_port
        );

        EmailService::new(smtp_host, smtp_port)
    }

    /// 创建 SmsService bean，使用自定义名称
    ///
    /// #[bean("customSmsService")] - 指定自定义的 bean 名称
    /// 直接返回具体类型，无需包装 Result
    #[bean("customSmsService")]
    pub fn sms_service(&self) -> SmsService {
        let api_key = self.environment
            .get_string("sms.api.key")
            .unwrap_or_else(|| "default-key".to_string());

        let api_endpoint = self.environment
            .get_string("sms.api.endpoint")
            .unwrap_or_else(|| "https://api.sms.example.com".to_string());

        tracing::info!(
            "📦 Creating SmsService bean with endpoint={}",
            api_endpoint
        );

        SmsService::new(api_key, api_endpoint)
    }

    /// 创建 NotificationService bean，依赖于其他 beans
    ///
    /// 展示如何在 bean 方法中通过 ApplicationContext 注入其他 beans
    /// 可以使用 get_bean() 或 get_bean_by_type() 获取依赖
    ///
    /// 返回 Result 类型，框架会自动处理错误传播
    #[bean]
    pub fn notification_service(&self) -> ContainerResult<NotificationService> {
        self.context.get_bean_by_type::<AppConfig>().map(|app_config| {
            tracing::info!(
                "📦 Creating NotificationService bean with app name: {}",
                app_config.name
            );
        });
        Ok(NotificationService::new(
            self.context.get_bean_by_type::<EmailService>()?,
            self.context.get_bean_by_type::<SmsService>()?,
        ))
    }

    /// 创建一个单例的缓存服务
    ///
    /// 展示如何创建一个简单的配置 bean
    /// 直接返回 HashMap，框架自动包装
    #[bean("appCache")]
    pub fn cache_service(&self) -> std::collections::HashMap<String, String> {
        tracing::info!("📦 Creating Cache bean (singleton)");

        let mut cache = std::collections::HashMap::new();
        cache.insert("version".to_string(), "1.0.0".to_string());
        cache.insert("env".to_string(), "development".to_string());

        cache
    }

    /// 创建一个原型作用域的计数器服务
    ///
    /// 展示如何使用 #[scope] 属性指定 bean 的作用域
    /// 每次获取都会创建新实例
    #[bean("prototypeCounter")]
    #[scope("prototype")]
    pub fn counter_service(&self) -> u32 {
        tracing::info!("📦 Creating Counter bean (prototype) - each get creates new instance");
        0
    }

    /// 创建一个延迟初始化的重量级服务
    ///
    /// 展示如何使用 #[lazy] 属性实现延迟初始化
    /// 只有在首次使用时才会创建，而不是在应用启动时创建
    #[bean("heavyService")]
    #[lazy]
    pub fn heavy_service(&self) -> String {
        tracing::info!("📦 Creating HeavyService bean (lazy) - only created when first accessed");
        "This is a heavy service that takes time to initialize".to_string()
    }

    /// 创建数据库连接池，带有 init 和 destroy 回调
    ///
    /// 展示如何使用 #[init] 和 #[destroy] 属性定义生命周期回调
    /// Bean 会在初始化时调用 init() 方法，销毁时调用 destroy() 方法
    #[bean("databasePool")]
    #[init]
    #[destroy]
    pub fn database_pool(&self) -> DatabaseConnectionPool {
        let pool_size = self.environment
            .get_i64("db.pool.size")
            .unwrap_or(10) as u32;

        tracing::info!("📦 Creating DatabaseConnectionPool bean");
        DatabaseConnectionPool::new(pool_size)
    }

    /// 创建缓存管理器，使用自定义的 init 和 destroy 方法名
    ///
    /// 展示如何使用 #[init("method_name")] 和 #[destroy("method_name")]
    /// 指定自定义的生命周期回调方法
    #[bean("cacheManager")]
    #[init("startup")]
    #[destroy("cleanup")]
    pub fn cache_manager(&self) -> CacheManager {
        let cache_name = self.environment
            .get_string("cache.name")
            .unwrap_or_else(|| "default-cache".to_string());

        tracing::info!("📦 Creating CacheManager bean");
        CacheManager::new(cache_name)
    }
}

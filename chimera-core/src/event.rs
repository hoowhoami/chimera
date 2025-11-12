use std::any::Any;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use std::collections::HashMap;

/// 事件 trait - 所有事件都必须实现此 trait
///
/// 类似 Spring 的 ApplicationEvent
pub trait Event: Any + Send + Sync {
    /// 获取事件名称
    fn event_name(&self) -> &str;

    /// 获取事件时间戳
    fn timestamp(&self) -> SystemTime;

    /// 转换为 Any 引用，用于类型转换
    fn as_any(&self) -> &dyn Any;
}

/// 应用启动完成事件
///
/// 在应用完全启动并初始化所有 Bean 后触发
#[derive(Debug, Clone)]
pub struct ApplicationStartedEvent {
    /// 应用名称
    pub app_name: String,
    /// 启动耗时（毫秒）
    pub startup_time_ms: u128,
    /// 事件时间戳
    pub timestamp: SystemTime,
}

impl ApplicationStartedEvent {
    pub fn new(app_name: String, startup_time_ms: u128) -> Self {
        Self {
            app_name,
            startup_time_ms,
            timestamp: SystemTime::now(),
        }
    }
}

impl Event for ApplicationStartedEvent {
    fn event_name(&self) -> &str {
        "ApplicationStartedEvent"
    }

    fn timestamp(&self) -> SystemTime {
        self.timestamp
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// 应用关闭事件
///
/// 在应用开始关闭时触发
#[derive(Debug, Clone)]
pub struct ApplicationShutdownEvent {
    /// 应用名称
    pub app_name: String,
    /// 事件时间戳
    pub timestamp: SystemTime,
}

impl ApplicationShutdownEvent {
    pub fn new(app_name: String) -> Self {
        Self {
            app_name,
            timestamp: SystemTime::now(),
        }
    }
}

impl Event for ApplicationShutdownEvent {
    fn event_name(&self) -> &str {
        "ApplicationShutdownEvent"
    }

    fn timestamp(&self) -> SystemTime {
        self.timestamp
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// 自定义应用事件
///
/// 允许用户发布自定义事件
#[derive(Debug, Clone)]
pub struct CustomEvent {
    /// 事件名称
    pub name: String,
    /// 事件数据
    pub data: Arc<dyn Any + Send + Sync>,
    /// 事件时间戳
    pub timestamp: SystemTime,
}

impl CustomEvent {
    pub fn new(name: String, data: Arc<dyn Any + Send + Sync>) -> Self {
        Self {
            name,
            data,
            timestamp: SystemTime::now(),
        }
    }
}

impl Event for CustomEvent {
    fn event_name(&self) -> &str {
        &self.name
    }

    fn timestamp(&self) -> SystemTime {
        self.timestamp
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// 事件监听器 trait
///
/// 类似 Spring 的 ApplicationListener
#[async_trait::async_trait]
pub trait EventListener: Send + Sync {
    /// 处理事件
    async fn on_event(&self, event: Arc<dyn Event>);

    /// 获取监听器名称（用于日志）
    fn listener_name(&self) -> &str {
        "AnonymousListener"
    }

    /// 是否支持该事件类型（可选实现，默认支持所有事件）
    fn supports_event(&self, event_name: &str) -> bool {
        let _ = event_name;
        true
    }
}

/// 类型化事件监听器 trait
///
/// 提供类型安全的事件处理
#[async_trait::async_trait]
pub trait TypedEventListener<E: Event>: Send + Sync {
    /// 处理特定类型的事件
    async fn on_event(&self, event: &E);
}

/// 事件发布器 trait
///
/// 类似 Spring 的 ApplicationEventPublisher
#[async_trait::async_trait]
pub trait EventPublisher: Send + Sync {
    /// 发布事件
    async fn publish_event(&self, event: Arc<dyn Event>);

    /// 注册事件监听器
    async fn register_listener(&self, listener: Arc<dyn EventListener>);

    /// 移除事件监听器
    async fn remove_listener(&self, listener_name: &str);

    /// 获取所有监听器数量
    async fn listener_count(&self) -> usize;
}

/// 简单事件发布器实现
///
/// 默认的事件发布器，支持同步事件分发
pub struct SimpleEventPublisher {
    /// 事件监听器列表
    listeners: RwLock<Vec<Arc<dyn EventListener>>>,
    /// 监听器名称到索引的映射
    listener_names: RwLock<HashMap<String, usize>>,
}

impl SimpleEventPublisher {
    /// 创建新的事件发布器
    pub fn new() -> Self {
        Self {
            listeners: RwLock::new(Vec::new()),
            listener_names: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for SimpleEventPublisher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl EventPublisher for SimpleEventPublisher {
    /// 发布事件到所有监听器
    async fn publish_event(&self, event: Arc<dyn Event>) {
        let listeners = self.listeners.read().await;
        let event_name = event.event_name();

        tracing::debug!(
            "Publishing event: {} to {} listener(s)",
            event_name,
            listeners.len()
        );

        // 遍历所有监听器并分发事件
        for listener in listeners.iter() {
            if listener.supports_event(event_name) {
                let event_clone = Arc::clone(&event);
                listener.on_event(event_clone).await;
            }
        }
    }

    /// 注册新的事件监听器
    async fn register_listener(&self, listener: Arc<dyn EventListener>) {
        let mut listeners = self.listeners.write().await;
        let mut names = self.listener_names.write().await;

        let listener_name = listener.listener_name().to_string();
        let index = listeners.len();

        listeners.push(listener);
        names.insert(listener_name.clone(), index);

        tracing::debug!("Registered event listener: {}", listener_name);
    }

    /// 移除事件监听器
    async fn remove_listener(&self, listener_name: &str) {
        let mut listeners = self.listeners.write().await;
        let mut names = self.listener_names.write().await;

        if let Some(&index) = names.get(listener_name) {
            listeners.remove(index);
            names.remove(listener_name);

            // 更新后续索引
            for (_, idx) in names.iter_mut() {
                if *idx > index {
                    *idx -= 1;
                }
            }

            tracing::debug!("Removed event listener: {}", listener_name);
        }
    }

    /// 获取当前监听器数量
    async fn listener_count(&self) -> usize {
        self.listeners.read().await.len()
    }
}

/// 异步事件发布器
///
/// 支持异步事件分发，不阻塞事件发布者
pub struct AsyncEventPublisher {
    inner: SimpleEventPublisher,
}

impl AsyncEventPublisher {
    pub fn new() -> Self {
        Self {
            inner: SimpleEventPublisher::new(),
        }
    }
}

impl Default for AsyncEventPublisher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl EventPublisher for AsyncEventPublisher {
    /// 异步发布事件（不等待监听器处理完成）
    async fn publish_event(&self, event: Arc<dyn Event>) {
        let listeners = self.inner.listeners.read().await;
        let event_name = event.event_name();

        tracing::debug!(
            "Publishing event asynchronously: {} to {} listener(s)",
            event_name,
            listeners.len()
        );

        // 克隆监听器列表，避免长时间持锁
        let listeners_clone: Vec<_> = listeners.iter()
            .filter(|l| l.supports_event(event_name))
            .map(Arc::clone)
            .collect();

        drop(listeners);

        // 异步分发事件到所有监听器
        for listener in listeners_clone {
            let event_clone = Arc::clone(&event);
            tokio::spawn(async move {
                listener.on_event(event_clone).await;
            });
        }
    }

    async fn register_listener(&self, listener: Arc<dyn EventListener>) {
        self.inner.register_listener(listener).await;
    }

    async fn remove_listener(&self, listener_name: &str) {
        self.inner.remove_listener(listener_name).await;
    }

    async fn listener_count(&self) -> usize {
        self.inner.listener_count().await
    }
}

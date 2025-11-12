use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;

/// 事件 trait - 所有事件都必须实现此 trait
///
/// 类似 Spring 的 ApplicationEvent
pub trait Event: Any + Send + Sync {
    /// 获取事件名称
    fn event_name(&self) -> &str;

    /// 获取事件时间戳
    fn timestamp(&self) -> SystemTime;

    /// 获取事件源（可选）
    /// 返回触发此事件的对象
    fn source(&self) -> Option<Arc<dyn Any + Send + Sync>> {
        None
    }

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

    /// 获取监听器名称（用于日志）
    fn listener_name(&self) -> &str {
        "AnonymousTypedListener"
    }
}

/// 类型化事件监听器适配器
///
/// 将 TypedEventListener<E> 适配为 EventListener
pub struct TypedEventListenerAdapter<E: Event + 'static, L: TypedEventListener<E>> {
    listener: Arc<L>,
    _phantom: std::marker::PhantomData<E>,
}

impl<E: Event + 'static, L: TypedEventListener<E>> TypedEventListenerAdapter<E, L> {
    pub fn new(listener: Arc<L>) -> Self {
        Self {
            listener,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<E: Event + 'static, L: TypedEventListener<E> + 'static> EventListener
    for TypedEventListenerAdapter<E, L>
{
    async fn on_event(&self, event: Arc<dyn Event>) {
        // 尝试将事件转换为具体类型
        if let Some(typed_event) = event.as_any().downcast_ref::<E>() {
            self.listener.on_event(typed_event).await;
        }
    }

    fn listener_name(&self) -> &str {
        self.listener.listener_name()
    }

    fn supports_event(&self, event_name: &str) -> bool {
        // 只支持特定类型的事件
        // 这里我们通过类型名称来判断
        let event_type_name = std::any::type_name::<E>();
        // 提取类型名称的最后一部分（去掉路径）
        let short_name = event_type_name
            .split("::")
            .last()
            .unwrap_or(event_type_name);
        event_name == short_name
    }
}

/// 事件发布器 trait
///
/// 类似 Spring 的 ApplicationEventPublisher
#[async_trait::async_trait]
pub trait EventPublisher: Any + Send + Sync {
    /// 发布事件
    async fn publish_event(&self, event: Arc<dyn Event>);

    /// 注册事件监听器
    async fn register_listener(&self, listener: Arc<dyn EventListener>);

    /// 移除事件监听器
    async fn remove_listener(&self, listener_name: &str);

    /// 获取所有监听器数量
    async fn listener_count(&self) -> usize;
}

/// AsyncEventPublisher 实现 CoreComponent
impl crate::container::CoreComponent for AsyncEventPublisher {
    fn core_bean_name() -> &'static str {
        crate::constants::EVENT_PUBLISHER_BEAN_NAME
    }

    fn get_from_context(
        context: &std::sync::Arc<crate::container::ApplicationContext>,
    ) -> std::sync::Arc<Self> {
        std::sync::Arc::clone(context.event_publisher())
    }
}

/// 异步事件发布器
///
/// 支持异步事件分发，不阻塞事件发布者
pub struct AsyncEventPublisher {
    /// 事件监听器列表
    listeners: RwLock<Vec<Arc<dyn EventListener>>>,
    /// 监听器名称到索引的映射
    listener_names: RwLock<HashMap<String, usize>>,
}

impl AsyncEventPublisher {
    pub fn new() -> Self {
        Self {
            listeners: RwLock::new(Vec::new()),
            listener_names: RwLock::new(HashMap::new()),
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
        let listeners = self.listeners.read().await;
        let event_name = event.event_name();

        tracing::debug!(
            "Publishing event asynchronously: {} to {} listener(s)",
            event_name,
            listeners.len()
        );

        // 克隆监听器列表，避免长时间持锁
        let listeners_clone: Vec<_> = listeners
            .iter()
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
        let mut listeners = self.listeners.write().await;
        let mut names = self.listener_names.write().await;

        let listener_name = listener.listener_name().to_string();
        let index = listeners.len();

        listeners.push(listener);
        names.insert(listener_name.clone(), index);

        tracing::debug!("Registered event listener: {}", listener_name);
    }

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

    async fn listener_count(&self) -> usize {
        self.listeners.read().await.len()
    }
}

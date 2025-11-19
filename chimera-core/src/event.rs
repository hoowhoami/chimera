use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use parking_lot::RwLock;

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
/// 默认同步执行，支持异步扩展
pub trait EventListener: Send + Sync {
    /// 处理事件（同步）
    fn on_event(&self, event: Arc<dyn Event>);

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
pub trait TypedEventListener<E: Event>: Send + Sync {
    /// 处理特定类型的事件
    fn on_event(&self, event: &E);

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

impl<E: Event + 'static, L: TypedEventListener<E> + 'static> EventListener
    for TypedEventListenerAdapter<E, L>
{
    fn on_event(&self, event: Arc<dyn Event>) {
        // 尝试将事件转换为具体类型
        if let Some(typed_event) = event.as_any().downcast_ref::<E>() {
            self.listener.on_event(typed_event);
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

/// 错误处理器类型
///
/// 用于处理监听器执行过程中的错误
pub type ErrorHandler = Arc<dyn Fn(&dyn EventListener, Arc<dyn Event>, &anyhow::Error) + Send + Sync>;

/// 事件多播器 trait
///
/// 类似 Spring 的 ApplicationEventMulticaster
/// 负责将事件传播到所有注册的监听器
pub trait ApplicationEventMulticaster: Send + Sync {
    /// 添加监听器
    fn add_listener(&self, listener: Arc<dyn EventListener>);

    /// 移除监听器
    fn remove_listener(&self, listener_name: &str);

    /// 移除所有监听器
    fn remove_all_listeners(&self);

    /// 广播事件到所有监听器
    ///
    /// 同步模式下，监听器抛出的异常会直接传递给发布线程（可能中断后续监听器执行）
    /// 可通过设置 errorHandler 统一处理异常，避免单个监听器异常影响整体
    fn multicast_event(&self, event: Arc<dyn Event>);

    /// 获取监听器数量
    fn listener_count(&self) -> usize;
}

/// 简单事件多播器实现
///
/// 默认同步执行，支持异步扩展
pub struct SimpleApplicationEventMulticaster {
    /// 事件监听器列表
    listeners: RwLock<Vec<Arc<dyn EventListener>>>,
    /// 监听器名称到索引的映射
    listener_names: RwLock<HashMap<String, usize>>,
    /// 错误处理器
    error_handler: RwLock<Option<ErrorHandler>>,
    /// 是否异步执行（如果为 true，会spawn到runtime）
    async_mode: bool,
}

impl SimpleApplicationEventMulticaster {
    /// 创建同步模式的多播器
    pub fn new() -> Self {
        Self {
            listeners: RwLock::new(Vec::new()),
            listener_names: RwLock::new(HashMap::new()),
            error_handler: RwLock::new(None),
            async_mode: false,
        }
    }

    /// 创建异步模式的多播器
    pub fn new_async() -> Self {
        Self {
            listeners: RwLock::new(Vec::new()),
            listener_names: RwLock::new(HashMap::new()),
            error_handler: RwLock::new(None),
            async_mode: true,
        }
    }

    /// 设置错误处理器
    pub fn set_error_handler<F>(&self, handler: F)
    where
        F: Fn(&dyn EventListener, Arc<dyn Event>, &anyhow::Error) + Send + Sync + 'static,
    {
        let mut error_handler = self.error_handler.write();
        *error_handler = Some(Arc::new(handler));
    }

    /// 移除错误处理器
    pub fn remove_error_handler(&self) {
        let mut error_handler = self.error_handler.write();
        *error_handler = None;
    }
}

impl Default for SimpleApplicationEventMulticaster {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplicationEventMulticaster for SimpleApplicationEventMulticaster {
    fn add_listener(&self, listener: Arc<dyn EventListener>) {
        let mut listeners = self.listeners.write();
        let mut names = self.listener_names.write();

        let listener_name = listener.listener_name().to_string();
        let index = listeners.len();

        listeners.push(listener);
        names.insert(listener_name.clone(), index);

        tracing::debug!("Added event listener: {}", listener_name);
    }

    fn remove_listener(&self, listener_name: &str) {
        let mut listeners = self.listeners.write();
        let mut names = self.listener_names.write();

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

    fn remove_all_listeners(&self) {
        let mut listeners = self.listeners.write();
        let mut names = self.listener_names.write();

        listeners.clear();
        names.clear();

        tracing::debug!("Removed all event listeners");
    }

    fn multicast_event(&self, event: Arc<dyn Event>) {
        let listeners = self.listeners.read();
        let event_name = event.event_name();

        tracing::debug!(
            "Multicasting event: {} to {} listener(s) (async_mode: {})",
            event_name,
            listeners.len(),
            self.async_mode
        );

        // 克隆监听器列表，避免长时间持锁
        let listeners_clone: Vec<_> = listeners
            .iter()
            .filter(|l| l.supports_event(event_name))
            .map(Arc::clone)
            .collect();

        drop(listeners);

        // 获取错误处理器
        let error_handler = self.error_handler.read().clone();

        if self.async_mode {
            // 异步模式：spawn到runtime
            for listener in listeners_clone {
                let event_clone = Arc::clone(&event);
                let error_handler_clone = error_handler.clone();

                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                    handle.spawn(async move {
                        if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            listener.on_event(event_clone.clone());
                        })) {
                            let err = anyhow::anyhow!("Listener panicked: {:?}", e);
                            if let Some(handler) = error_handler_clone {
                                handler(listener.as_ref(), event_clone, &err);
                            } else {
                                tracing::error!(
                                    "Listener '{}' panicked while handling event '{}': {:?}",
                                    listener.listener_name(),
                                    event_clone.event_name(),
                                    err
                                );
                            }
                        }
                    });
                } else {
                    // 没有runtime，降级为同步执行
                    tracing::warn!("No tokio runtime available, falling back to sync execution");
                    self.invoke_listener(&listener, Arc::clone(&event), error_handler.as_ref());
                }
            }
        } else {
            // 同步模式：顺序执行
            for listener in listeners_clone {
                self.invoke_listener(&listener, Arc::clone(&event), error_handler.as_ref());
            }
        }
    }

    fn listener_count(&self) -> usize {
        self.listeners.read().len()
    }
}

impl SimpleApplicationEventMulticaster {
    /// 调用单个监听器
    fn invoke_listener(
        &self,
        listener: &Arc<dyn EventListener>,
        event: Arc<dyn Event>,
        error_handler: Option<&ErrorHandler>,
    ) {
        // 使用 catch_unwind 捕获 panic
        if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            listener.on_event(event.clone());
        })) {
            let err = anyhow::anyhow!("Listener panicked: {:?}", e);
            if let Some(handler) = error_handler {
                handler(listener.as_ref(), event, &err);
            } else {
                // 没有错误处理器，重新抛出
                tracing::error!(
                    "Listener '{}' panicked while handling event '{}': {:?}",
                    listener.listener_name(),
                    event.event_name(),
                    err
                );
                std::panic::resume_unwind(e);
            }
        }
    }
}

/// 事件发布器
///
/// 类似 Spring 的 ApplicationEventPublisher
/// 简化的发布接口，内部使用 ApplicationEventMulticaster
///
/// 注意：此类不应由用户直接构造，只能通过容器依赖注入获取
pub struct ApplicationEventPublisher {
    multicaster: Arc<dyn ApplicationEventMulticaster>,
}

impl ApplicationEventPublisher {
    /// 创建发布器（内部方法，仅供 ApplicationContext 使用）
    ///
    /// 用户不应直接调用此方法，应通过依赖注入获取
    pub(crate) fn new(multicaster: Arc<dyn ApplicationEventMulticaster>) -> Self {
        Self { multicaster }
    }

    /// 发布事件
    pub fn publish_event(&self, event: Arc<dyn Event>) {
        self.multicaster.multicast_event(event);
    }

    /// 获取多播器
    pub fn multicaster(&self) -> &Arc<dyn ApplicationEventMulticaster> {
        &self.multicaster
    }

    /// 添加监听器
    pub fn add_listener(&self, listener: Arc<dyn EventListener>) {
        self.multicaster.add_listener(listener);
    }

    /// 移除监听器
    pub fn remove_listener(&self, listener_name: &str) {
        self.multicaster.remove_listener(listener_name);
    }

    /// 获取监听器数量
    pub fn listener_count(&self) -> usize {
        self.multicaster.listener_count()
    }
}

/// ApplicationEventPublisher 实现 CoreComponent
impl crate::container::CoreComponent for ApplicationEventPublisher {
    fn core_bean_name() -> &'static str {
        crate::constants::EVENT_PUBLISHER_BEAN_NAME
    }

    fn get_from_context(
        context: &std::sync::Arc<crate::container::ApplicationContext>,
    ) -> std::sync::Arc<Self> {
        std::sync::Arc::clone(context.event_publisher())
    }
}

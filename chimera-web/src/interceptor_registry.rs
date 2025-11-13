//! 处理器拦截器编译时注册机制
//!
//! 仿照 controller 的注册方式，使用 inventory 实现编译时自动收集

use crate::interceptor::{HandlerInterceptor, InterceptorRegistry};

/// 处理器拦截器注册信息
pub struct HandlerInterceptorRegistration {
    pub name: &'static str,
    pub create: fn() -> Box<dyn HandlerInterceptor>,
}

impl HandlerInterceptorRegistration {
    pub fn new(name: &'static str, create: fn() -> Box<dyn HandlerInterceptor>) -> Self {
        Self { name, create }
    }
}

inventory::collect!(HandlerInterceptorRegistration);

/// 获取所有注册的处理器拦截器
pub fn get_all_handler_interceptors() -> Vec<&'static HandlerInterceptorRegistration> {
    inventory::iter::<HandlerInterceptorRegistration>
        .into_iter()
        .collect()
}

/// 构建拦截器注册表 - 使用编译时收集的处理器拦截器
pub fn build_interceptor_registry_from_inventory() -> InterceptorRegistry {
    let mut registry = InterceptorRegistry::new();

    tracing::info!("🔍 Discovering handler interceptors from inventory...");

    for interceptor_info in get_all_handler_interceptors() {
        let interceptor = (interceptor_info.create)();
        tracing::info!(
            "✅ Auto-registered handler interceptor: {}",
            interceptor_info.name
        );
        registry.register_boxed(interceptor);
    }

    tracing::info!(
        "✅ Handler interceptor discovery completed: {} interceptors registered",
        registry.len()
    );

    registry
}

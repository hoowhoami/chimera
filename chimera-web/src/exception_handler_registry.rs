//! 异常处理器编译时注册机制
//!
//! 仿照 controller 的注册方式，使用 inventory 实现编译时自动收集

use crate::exception_handler::{GlobalExceptionHandler, GlobalExceptionHandlerRegistry};

/// 异常处理器注册信息
pub struct ExceptionHandlerRegistration {
    pub name: &'static str,
    pub create: fn() -> Box<dyn GlobalExceptionHandler>,
}

impl ExceptionHandlerRegistration {
    pub fn new(name: &'static str, create: fn() -> Box<dyn GlobalExceptionHandler>) -> Self {
        Self { name, create }
    }
}

inventory::collect!(ExceptionHandlerRegistration);

/// 获取所有注册的异常处理器
pub fn get_all_exception_handlers() -> Vec<&'static ExceptionHandlerRegistration> {
    inventory::iter::<ExceptionHandlerRegistration>
        .into_iter()
        .collect()
}

/// 构建异常处理器注册表 - 使用编译时收集的处理器
pub fn build_exception_handler_registry_from_inventory() -> GlobalExceptionHandlerRegistry {
    let mut registry = GlobalExceptionHandlerRegistry::new();

    tracing::info!("🔍 Discovering exception handlers from inventory...");

    for handler_info in get_all_exception_handlers() {
        let handler = (handler_info.create)();
        tracing::info!(
            "✅ Auto-registered exception handler: {}",
            handler_info.name
        );
        registry.register_boxed(handler);
    }

    tracing::info!(
        "✅ Exception handler discovery completed: {} handlers registered",
        registry.len()
    );

    registry
}

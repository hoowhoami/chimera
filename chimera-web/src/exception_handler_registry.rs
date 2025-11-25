//! 异常处理器编译时注册机制
//!
//! 仿照 controller 的注册方式，使用 inventory 实现编译时自动收集

use crate::exception_handler::{GlobalExceptionHandler, GlobalExceptionHandlerRegistry};
use chimera_core::{ApplicationContext, Container};
use std::sync::Arc;

/// 异常处理器注册信息
pub struct ExceptionHandlerRegistration {
    pub name: &'static str,
    pub bean_name: &'static str,
    /// 类型转换函数：将 Arc<dyn Any> 转换为 Arc<dyn GlobalExceptionHandler>
    pub cast_fn: fn(Arc<dyn std::any::Any + Send + Sync>) -> Option<Arc<dyn GlobalExceptionHandler>>,
}

impl ExceptionHandlerRegistration {
    pub const fn new(
        name: &'static str,
        bean_name: &'static str,
        cast_fn: fn(Arc<dyn std::any::Any + Send + Sync>) -> Option<Arc<dyn GlobalExceptionHandler>>,
    ) -> Self {
        Self {
            name,
            bean_name,
            cast_fn,
        }
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
pub async fn build_exception_handler_registry_from_inventory(
    context: &Arc<ApplicationContext>,
) -> chimera_core::Result<GlobalExceptionHandlerRegistry> {
    let mut registry = GlobalExceptionHandlerRegistry::new();

    tracing::info!("Discovering exception handlers from inventory...");

    for handler_info in get_all_exception_handlers() {
        // 从容器中获取已经创建好的bean实例
        match context.get_bean(handler_info.bean_name) {
            Ok(bean_any) => {
                // 使用类型转换函数将 Arc<dyn Any> 转换为 Arc<dyn GlobalExceptionHandler>
                match (handler_info.cast_fn)(bean_any) {
                    Some(handler) => {
                        tracing::info!(
                            "Auto-registered exception handler: {} (bean: {})",
                            handler_info.name,
                            handler_info.bean_name
                        );
                        registry.register_arc(handler);
                    }
                    None => {
                        tracing::error!(
                            "Failed to cast bean '{}' to GlobalExceptionHandler",
                            handler_info.bean_name
                        );
                        return Err(anyhow::anyhow!(
                            "Failed to cast bean '{}' to GlobalExceptionHandler",
                            handler_info.bean_name
                        ));
                    }
                }
            }
            Err(e) => {
                tracing::error!(
                    "Failed to get exception handler bean '{}': {}",
                    handler_info.bean_name,
                    e
                );
                return Err(anyhow::anyhow!(
                    "Failed to get exception handler bean '{}': {}",
                    handler_info.bean_name, e
                ));
            }
        }
    }

    tracing::info!(
        "Exception handler discovery completed: {} handlers registered",
        registry.len()
    );

    Ok(registry)
}

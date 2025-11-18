//! Chimera AOP 过程宏
//!
//! 提供 AOP 相关的过程宏，包括：
//! - `#[Aspect]` - 定义切面
//! - `#[pointcut]` - 定义切点表达式（用于结构体属性）

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, ItemStruct, Meta};

mod aspect;
mod utils;
mod aop_method;

/// `#[Aspect]` 宏
///
/// 将结构体标记为切面，并自动实现 Aspect trait
///
/// 使用示例：
/// ```ignore
/// use chimera_aop_macros::Aspect;
/// use chimera_aop::prelude::*;
///
/// #[derive(Aspect)]
/// #[pointcut("execution(* UserService.*(..))")]
/// pub struct LoggingAspect;
///
/// impl LoggingAspect {
///     pub async fn before(&self, jp: &JoinPoint) {
///         tracing::info!("→ {}", jp.signature());
///     }
/// }
/// ```
#[proc_macro_derive(Aspect, attributes(pointcut))]
pub fn derive_aspect(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    aspect::impl_aspect_derive(&input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// `#[aop]` 属性宏
///
/// 为方法自动应用 AOP 切面，类似 Spring Boot 的声明式 AOP
///
/// 使用示例：
/// ```ignore
/// impl UserService {
///     #[aop("UserService", "get_user")]
///     pub async fn get_user(&self, id: u32) -> Result<User, Error> {
///         // 业务逻辑
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn aop(attr: TokenStream, item: TokenStream) -> TokenStream {
    use syn::{parse_macro_input, ItemFn};

    let item_fn = parse_macro_input!(item as ItemFn);

    // 简单解析：期望两个字符串字面量 "Type", "method"
    let (target_type, method_name) = if !attr.is_empty() {
        let attr_str = attr.to_string();
        let parts: Vec<&str> = attr_str.split(',').map(|s| s.trim().trim_matches('"')).collect();

        match parts.len() {
            2 => (Some(parts[0].to_string()), Some(parts[1].to_string())),
            1 => (Some(parts[0].to_string()), None),
            _ => (None, None),
        }
    } else {
        (None, None)
    };

    aop_method::impl_aop_method(target_type, method_name, item_fn).into()
}

/// `#[before]` 属性宏
///
/// 标记方法为前置通知
#[proc_macro_attribute]
pub fn before(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // 目前只是简单地返回原始代码
    // 未来可以自动注册前置通知
    item
}

/// `#[after]` 属性宏
///
/// 标记方法为后置通知
#[proc_macro_attribute]
pub fn after(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// `#[around]` 属性宏
///
/// 标记方法为环绕通知
#[proc_macro_attribute]
pub fn around(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// `#[after_returning]` 属性宏
///
/// 标记方法为返回后通知
#[proc_macro_attribute]
pub fn after_returning(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// `#[after_throwing]` 属性宏
///
/// 标记方法为异常通知
#[proc_macro_attribute]
pub fn after_throwing(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

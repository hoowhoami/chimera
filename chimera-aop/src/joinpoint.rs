//! 连接点（JoinPoint）定义
//!
//! 连接点表示程序执行的特定点，比如方法调用

use std::any::Any;
use std::fmt;
use std::sync::Arc;
use std::time::Instant;

/// 连接点信息
///
/// 包含方法调用时的上下文信息
#[derive(Clone)]
pub struct JoinPoint {
    /// 目标类型名称
    pub target_type: &'static str,

    /// 方法名称
    pub method_name: &'static str,

    /// 方法参数（如果需要）
    pub args: Option<Arc<dyn Any + Send + Sync>>,

    /// 调用时间戳
    pub timestamp: Instant,
}

impl JoinPoint {
    /// 创建新的连接点
    pub fn new(target_type: &'static str, method_name: &'static str) -> Self {
        Self {
            target_type,
            method_name,
            args: None,
            timestamp: Instant::now(),
        }
    }

    /// 设置参数
    pub fn with_args<T: Any + Send + Sync + 'static>(mut self, args: T) -> Self {
        self.args = Some(Arc::new(args));
        self
    }

    /// 获取完整的方法签名
    pub fn signature(&self) -> String {
        format!("{}::{}", self.target_type, self.method_name)
    }

    /// 获取目标类型名称
    pub fn get_target_type(&self) -> &'static str {
        self.target_type
    }

    /// 获取方法名称
    pub fn get_method_name(&self) -> &'static str {
        self.method_name
    }

    /// 获取调用时间戳
    pub fn get_timestamp(&self) -> &Instant {
        &self.timestamp
    }

    /// 尝试获取参数
    pub fn get_args<T: Any + Send + Sync>(&self) -> Option<&T> {
        self.args.as_ref()?.downcast_ref::<T>()
    }
}

impl fmt::Debug for JoinPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JoinPoint")
            .field("target_type", &self.target_type)
            .field("method_name", &self.method_name)
            .field("signature", &self.signature())
            .field("timestamp", &self.timestamp)
            .finish()
    }
}

impl fmt::Display for JoinPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.signature())
    }
}

/// 环绕通知的执行链
///
/// 允许切面控制是否继续执行目标方法
pub struct ProceedingJoinPoint<T, E> {
    /// 连接点信息
    pub join_point: JoinPoint,

    /// 继续执行的函数
    proceed_fn: Option<Box<dyn FnOnce() -> Result<T, E> + Send>>,
}

impl<T, E> ProceedingJoinPoint<T, E> {
    /// 创建新的环绕连接点
    pub fn new<F>(join_point: JoinPoint, proceed_fn: F) -> Self
    where
        F: FnOnce() -> Result<T, E> + Send + 'static,
    {
        Self {
            join_point,
            proceed_fn: Some(Box::new(proceed_fn)),
        }
    }

    /// 继续执行目标方法
    ///
    /// 注意：此方法只能调用一次
    pub fn proceed(mut self) -> Result<T, E> {
        if let Some(func) = self.proceed_fn.take() {
            func()
        } else {
            panic!("ProceedingJoinPoint::proceed() can only be called once");
        }
    }

    /// 获取连接点信息
    pub fn get_join_point(&self) -> &JoinPoint {
        &self.join_point
    }
}

impl<T, E> fmt::Debug for ProceedingJoinPoint<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProceedingJoinPoint")
            .field("join_point", &self.join_point)
            .field("has_proceed_fn", &self.proceed_fn.is_some())
            .finish()
    }
}

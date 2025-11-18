//! 切面注册表
//!
//! 负责管理和执行所有切面

use crate::{Aspect, JoinPoint};
use once_cell::sync::Lazy;
use std::sync::Arc;

/// 全局 AOP 注册表
///
/// 应用启动时自动初始化，加载所有通过 inventory 注册的切面
static GLOBAL_ASPECT_REGISTRY: Lazy<Arc<AspectRegistry>> = Lazy::new(|| {
    let mut registry = AspectRegistry::new();
    registry.auto_load_aspects();
    Arc::new(registry)
});

/// 获取全局 AOP 注册表
///
/// 使用示例：
/// ```ignore
/// use chimera_aop::get_global_registry;
///
/// let registry = get_global_registry();
/// registry.execute_with_aspects(jp, || {
///     // your code here
/// }).await
/// ```
pub fn get_global_registry() -> &'static Arc<AspectRegistry> {
    &GLOBAL_ASPECT_REGISTRY
}

/// 切面注册表
///
/// 管理所有注册的切面，并提供执行通知的方法
pub struct AspectRegistry {
    aspects: Vec<Arc<dyn Aspect>>,
}

impl AspectRegistry {
    /// 创建新的切面注册表
    pub fn new() -> Self {
        Self {
            aspects: Vec::new(),
        }
    }

    /// 注册切面
    pub fn register(&mut self, aspect: Arc<dyn Aspect>) {
        tracing::debug!("Registering aspect: {}", aspect.name());
        self.aspects.push(aspect);
    }

    /// 批量注册切面
    pub fn register_all(&mut self, aspects: impl IntoIterator<Item = Arc<dyn Aspect>>) {
        for aspect in aspects {
            self.register(aspect);
        }
    }

    /// 获取匹配指定连接点的所有切面
    pub fn get_matching_aspects(&self, join_point: &JoinPoint) -> Vec<Arc<dyn Aspect>> {
        self.aspects
            .iter()
            .filter(|aspect| aspect.pointcut().matches(join_point))
            .cloned()
            .collect()
    }

    /// 执行前置通知
    pub async fn execute_before(&self, join_point: &JoinPoint) {
        let aspects = self.get_matching_aspects(join_point);
        for aspect in aspects {
            aspect.before(join_point).await;
        }
    }

    /// 执行后置通知
    pub async fn execute_after(&self, join_point: &JoinPoint) {
        let aspects = self.get_matching_aspects(join_point);
        for aspect in aspects {
            aspect.after(join_point).await;
        }
    }

    /// 执行返回后通知
    pub async fn execute_after_returning(&self, join_point: &JoinPoint) {
        let aspects = self.get_matching_aspects(join_point);
        for aspect in aspects {
            aspect.after_returning(join_point).await;
        }
    }

    /// 执行异常通知
    pub async fn execute_after_throwing(&self, join_point: &JoinPoint, error_msg: &str) {
        let aspects = self.get_matching_aspects(join_point);
        for aspect in aspects {
            aspect.after_throwing(join_point, error_msg).await;
        }
    }

    /// 执行带AOP的方法调用
    ///
    /// 这个方法会按顺序执行：
    /// 1. 前置通知
    /// 2. 目标方法
    /// 3. 返回后通知或异常通知
    /// 4. 后置通知
    pub async fn execute_with_aspects<T: 'static, E, F>(
        &self,
        join_point: JoinPoint,
        func: F,
    ) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
        E: std::error::Error + 'static,
    {
        // 前置通知
        self.execute_before(&join_point).await;

        // 执行目标方法
        let result = func();

        // 根据结果执行不同的通知
        match &result {
            Ok(_value) => {
                // 返回后通知
                self.execute_after_returning(&join_point).await;
            }
            Err(error) => {
                // 异常通知（将错误转换为字符串）
                let error_str = error.to_string();
                self.execute_after_throwing(&join_point, &error_str).await;
            }
        }

        // 后置通知（无论成功还是失败都执行）
        self.execute_after(&join_point).await;

        result
    }

    /// 获取注册的切面数量
    pub fn len(&self) -> usize {
        self.aspects.len()
    }

    /// 检查是否没有注册任何切面
    pub fn is_empty(&self) -> bool {
        self.aspects.is_empty()
    }

    /// 清除所有切面
    pub fn clear(&mut self) {
        self.aspects.clear();
    }

    /// 从 inventory 自动加载所有注册的切面
    ///
    /// 这个方法会扫描所有通过 #[derive(Aspect)] 宏自动注册的切面
    /// 并将它们添加到切面注册表中
    ///
    /// 使用示例：
    /// ```ignore
    /// let mut registry = AspectRegistry::new();
    /// registry.auto_load_aspects();
    /// ```
    pub fn auto_load_aspects(&mut self) {
        let registrations: Vec<_> = crate::aspect::get_all_aspect_registrations().collect();
        tracing::info!("Auto-loading {} aspect(s) from registry", registrations.len());

        for registration in registrations {
            tracing::debug!(
                "  ├─ Loading aspect: {} with pointcut: {}",
                registration.name,
                registration.pointcut_expr
            );

            let aspect = registration.create_instance();
            self.register(aspect);
        }

        tracing::info!("Auto-loaded {} aspect(s)", self.len());
    }
}

impl Default for AspectRegistry {
    fn default() -> Self {
        Self::new()
    }
}

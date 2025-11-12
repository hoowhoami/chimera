use crate::ContainerResult;
use std::any::Any;

/// 生命周期管理 trait
/// 类似 Spring 的 @PostConstruct 和 @PreDestroy
pub trait Lifecycle: Any + Send + Sync {
    /// 初始化回调
    /// 在 Bean 创建并设置所有依赖后调用
    fn init(&mut self) -> ContainerResult<()> {
        Ok(())
    }

    /// 销毁回调
    /// 在容器关闭或 Bean 被移除时调用
    fn destroy(&mut self) -> ContainerResult<()> {
        Ok(())
    }
}

/// 自动为所有类型实现空的 Lifecycle（可选实现）
impl<T: Any + Send + Sync> Lifecycle for T {}

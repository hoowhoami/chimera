/// Bean 的作用域
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// 单例模式 - 容器中只有一个实例
    Singleton,

    /// 原型模式 - 每次请求都创建新实例
    Prototype,
}

impl Default for Scope {
    fn default() -> Self {
        Scope::Singleton
    }
}

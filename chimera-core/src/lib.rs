// chimera-core: 类似 Spring Boot 的依赖注入容器
//
// 提供类型安全的依赖注入功能，支持：
// - 单例和原型作用域
// - 构造函数注入
// - 生命周期管理（init/destroy 回调）
// - 自动装配（通过宏）

pub mod container;
pub mod error;
pub mod scope;
pub mod bean;
pub mod lifecycle;
pub mod component;
pub mod utils;
pub mod config;
pub mod app;
pub mod logging;

// 重新导出常用类型
pub use container::{Container, ApplicationContext};
pub use error::{ContainerError, ContainerResult, ApplicationError, ApplicationResult};
pub use scope::Scope;
pub use bean::{Bean, BeanDefinition, BeanFactory};
pub use lifecycle::Lifecycle;
pub use component::Component;
pub use config::{
    Environment, PropertySource, ConfigValue,
    EnvironmentPropertySource, TomlPropertySource, MapPropertySource,
};
pub use app::ChimeraApplication;
pub use logging::{LoggingConfig, LogLevel, LogFormat};

// 导出 async_trait，供宏使用
pub use async_trait;

/// Prelude 模块，包含常用的 traits 和类型
pub mod prelude {
    pub use crate::container::{Container, ApplicationContext};
    pub use crate::error::{ContainerError, ContainerResult, ApplicationError, ApplicationResult};
    pub use crate::scope::Scope;
    pub use crate::bean::{Bean, BeanDefinition, BeanFactory};
    pub use crate::lifecycle::Lifecycle;
    pub use crate::component::Component;
    pub use crate::config::{
        self,
        Environment, PropertySource, ConfigValue,
        EnvironmentPropertySource, TomlPropertySource, MapPropertySource,
    };
    pub use crate::app::ChimeraApplication;
    pub use crate::logging::{LoggingConfig, LogLevel, LogFormat};
    pub use crate::utils;
}

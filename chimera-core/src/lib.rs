// chimera-core: 类似 Spring Boot 的依赖注入容器
//
// 提供类型安全的依赖注入功能，支持：
// - 单例和原型作用域
// - 构造函数注入
// - 生命周期管理（init/destroy 回调）
// - 自动装配（通过宏）

pub mod app;
pub mod bean;
pub mod component;
pub mod config;
pub mod constants;
pub mod container;
pub mod error;
pub mod event;
pub mod lifecycle;
pub mod logging;
pub mod scope;
pub mod utils;

// 重新导出常用类型
pub use app::ChimeraApplication;
pub use bean::{Bean, BeanDefinition, BeanFactory};
pub use component::Component;
pub use component::{ComponentRegistry, ConfigurationPropertiesRegistry, EventListenerRegistry};
pub use config::{
    ConfigValue, Environment, EnvironmentPropertySource, MapPropertySource, PropertySource,
    TomlPropertySource,
};
pub use constants::*;
pub use container::{
    ApplicationContext, ApplicationContextBuilder, Container, CoreComponent, ShutdownHook,
};
pub use error::{ApplicationError, ApplicationResult, ContainerError, ContainerResult};
pub use event::{
    ApplicationShutdownEvent, ApplicationStartedEvent, AsyncEventPublisher, Event, EventListener,
    EventPublisher, TypedEventListener, TypedEventListenerAdapter,
};
pub use lifecycle::Lifecycle;
pub use logging::{LogFormat, LogLevel, LoggingConfig};
pub use scope::Scope;

// 导出 async_trait，供宏使用
pub use async_trait;

/// Prelude 模块，包含常用的 traits 和类型
pub mod prelude {
    pub use crate::app::ChimeraApplication;
    pub use crate::bean::{Bean, BeanDefinition, BeanFactory};
    pub use crate::component::Component;
    pub use crate::config::{
        self, ConfigValue, Environment, EnvironmentPropertySource, MapPropertySource,
        PropertySource, TomlPropertySource,
    };
    pub use crate::container::{ApplicationContext, Container};
    pub use crate::error::{ApplicationError, ApplicationResult, ContainerError, ContainerResult};
    pub use crate::event::{
        ApplicationShutdownEvent, ApplicationStartedEvent, AsyncEventPublisher, Event,
        EventListener, EventPublisher, TypedEventListener, TypedEventListenerAdapter,
    };
    pub use crate::lifecycle::Lifecycle;
    pub use crate::logging::{LogFormat, LogLevel, LoggingConfig};
    pub use crate::scope::Scope;
    pub use crate::utils;
}

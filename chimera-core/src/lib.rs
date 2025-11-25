// chimera-core: 类似 Spring Boot 的依赖注入容器
//
// 提供类型安全的依赖注入功能，支持：
// - 单例和原型作用域
// - 构造函数注入
// - 生命周期管理（init/destroy 回调）
// - 自动装配（通过宏）

pub mod app;
pub mod bean;
pub mod bean_factory;
pub mod component;
pub mod config;
pub mod constants;
pub mod context;
pub mod error;
pub mod event;
pub mod lifecycle;
pub mod logging;
pub mod plugin;
pub mod scope;
pub mod utils;

// Helper trait for init/destroy callbacks
// Allows both () and Result<()> return types
pub trait IntoResult {
    fn into_result(self) -> Result<()>;
}

impl IntoResult for () {
    fn into_result(self) -> Result<()> {
        Ok(())
    }
}

impl IntoResult for Result<()> {
    fn into_result(self) -> Result<()> {
        self
    }
}

// 重新导出常用类型
pub use app::{ChimeraApplication, RunningApplication};
pub use bean::{Bean, BeanDefinition, FactoryBean};
pub use bean_factory::{
    BeanFactory, BeanFactoryExt, ConfigurableBeanFactory, ConfigurableListableBeanFactory,
    DefaultListableBeanFactory, ListableBeanFactory,
};
pub use component::Component;
pub use component::{ComponentRegistry, ConfigurationPropertiesRegistry, EventListenerRegistry};
pub use config::{
    ConfigValue, Environment, EnvironmentPropertySource, MapPropertySource, PropertySource,
    TomlPropertySource,
};
pub use constants::*;
pub use context::{ApplicationContext, ApplicationContextBuilder, Container, ShutdownHook};
pub use error::Result;
pub use event::{
    ApplicationEventMulticaster, ApplicationEventPublisher, ApplicationShutdownEvent,
    ApplicationStartedEvent, ErrorHandler, Event, EventListener,
    SimpleApplicationEventMulticaster, TypedEventListener, TypedEventListenerAdapter,
};
pub use lifecycle::{
    BeanFactoryPostProcessor, BeanFactoryPostProcessorMarker, BeanPostProcessor,
    BeanPostProcessorMarker, SmartInitializingSingleton, SmartInitializingSingletonMarker,
};
pub use logging::{LogFormat, LogLevel, LoggingConfig};
pub use scope::Scope;

// 导出 async_trait 和 inventory，供宏使用
pub use async_trait;
pub use inventory;

// 导出插件相关
pub use plugin::{ApplicationPlugin, PluginRegistry, PluginSubmission, load_plugins};

/// Prelude 模块，包含常用的 traits 和类型
pub mod prelude {
    pub use crate::app::{ChimeraApplication, RunningApplication};
    pub use crate::bean::{Bean, BeanDefinition, FactoryBean};
    pub use crate::bean_factory::{
        BeanFactory, BeanFactoryExt, ConfigurableBeanFactory, ConfigurableListableBeanFactory,
        DefaultListableBeanFactory, ListableBeanFactory,
    };
    pub use crate::component::Component;
    pub use crate::config::{
        self, ConfigValue, Environment, EnvironmentPropertySource, MapPropertySource,
        PropertySource, TomlPropertySource,
    };
    pub use crate::context::{ApplicationContext, Container};
    pub use crate::error::Result;
    pub use crate::event::{
        ApplicationEventMulticaster, ApplicationEventPublisher, ApplicationShutdownEvent,
        ApplicationStartedEvent, Event, EventListener, SimpleApplicationEventMulticaster,
        TypedEventListener, TypedEventListenerAdapter,
    };
    pub use crate::lifecycle::{
        BeanFactoryPostProcessor, BeanPostProcessor, SmartInitializingSingleton,
        SmartInitializingSingletonMarker,
    };
    pub use crate::logging::{LogFormat, LogLevel, LoggingConfig};
    pub use crate::plugin::{ApplicationPlugin, PluginRegistry, load_plugins};
    pub use crate::scope::Scope;
    pub use crate::utils;
    // Re-export anyhow for convenience
    pub use anyhow::{anyhow, Context};
}

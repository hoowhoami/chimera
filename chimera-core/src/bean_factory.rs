//! Bean Factory - 核心容器接口
//!
//! 参考 Spring 的 BeanFactory 架构设计

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

use crate::{
    bean::BeanDefinition,
    error::{ContainerError, ContainerResult},
    lifecycle::BeanPostProcessor,
    utils::dependency::CreationTracker,
};

/// BeanFactory - 最基础的容器接口
///
/// 提供基本的 Bean 访问功能，类似 Spring 的 BeanFactory
///
/// 注意：此 trait 不包含泛型方法，因此可以作为 trait object 使用
pub trait BeanFactory: Send + Sync {
    /// 通过名称获取 Bean
    fn get_bean(&self, name: &str) -> ContainerResult<Arc<dyn Any + Send + Sync>>;

    /// 检查是否包含指定名称的 Bean
    fn contains_bean(&self, name: &str) -> bool;
}

/// BeanFactoryExt - BeanFactory 的扩展 trait
///
/// 提供泛型方法，不能作为 trait object 使用
pub trait BeanFactoryExt: BeanFactory {
    /// 通过类型获取 Bean
    fn get_bean_by_type<T: Any + Send + Sync>(&self) -> ContainerResult<Arc<T>>;

    /// 检查是否包含指定类型的 Bean
    fn contains_bean_by_type<T: Any + Send + Sync>(&self) -> bool;
}

/// ListableBeanFactory - 可列举的 Bean 工厂
///
/// 扩展 BeanFactory，提供列举所有 Bean 的能力
pub trait ListableBeanFactory: BeanFactory {
    /// 获取所有 Bean 的名称
    fn get_bean_names(&self) -> Vec<String>;

    /// 获取指定类型的所有 Bean 名称
    fn get_bean_names_for_type(&self, type_id: TypeId) -> Vec<String>;

    /// 获取 Bean 定义的数量
    fn get_bean_definition_count(&self) -> usize;
}

/// ConfigurableBeanFactory - 可配置的 Bean 工厂
///
/// 提供配置和管理 Bean 工厂的能力
pub trait ConfigurableBeanFactory: BeanFactory {
    /// 注册 Bean 定义
    fn register_bean_definition(&self, name: String, definition: BeanDefinition) -> ContainerResult<()>;

    /// 检查是否包含指定的 Bean 定义
    fn contains_bean_definition(&self, name: &str) -> bool;

    /// 移除 Bean 定义
    fn remove_bean_definition(&self, name: &str) -> ContainerResult<()>;

    /// 获取单个 Bean 定义
    fn get_bean_definition(&self, name: &str) -> ContainerResult<BeanDefinition>;

    /// 修改 Bean 定义
    fn modify_bean_definition<F>(&self, name: &str, modifier: F) -> ContainerResult<()>
    where
        F: FnOnce(&mut BeanDefinition);

    /// 添加 BeanPostProcessor
    fn add_bean_post_processor(&self, processor: Arc<dyn BeanPostProcessor>);

    /// 获取所有 BeanPostProcessor
    fn get_bean_post_processors(&self) -> Vec<Arc<dyn BeanPostProcessor>>;
}

/// ConfigurableListableBeanFactory - 可配置且可列举的 Bean 工厂
///
/// 结合了 ListableBeanFactory 和 ConfigurableBeanFactory 的功能
/// 这是 BeanFactoryPostProcessor 接收的参数类型
pub trait ConfigurableListableBeanFactory: ListableBeanFactory + ConfigurableBeanFactory {
    /// 预实例化所有单例 Bean
    fn preinstantiate_singletons(&self) -> ContainerResult<()>;

    /// 冻结配置（不再允许修改 Bean 定义）
    fn freeze_configuration(&self);

    /// 检查配置是否已冻结
    fn is_configuration_frozen(&self) -> bool;

    /// 销毁所有单例 Bean（调用 destroy 回调）
    fn destroy_singletons(&self) -> ContainerResult<()>;

    /// 获取所有 Bean 定义（用于依赖验证等）
    fn get_bean_definitions(&self) -> std::collections::HashMap<String, Vec<String>>;
}

/// DefaultListableBeanFactory - ConfigurableListableBeanFactory 的默认实现
///
/// 这是实际的 Bean 容器实现，类似 Spring 的 DefaultListableBeanFactory
pub struct DefaultListableBeanFactory {
    /// Bean 定义存储
    definitions: RwLock<HashMap<String, BeanDefinition>>,

    /// 单例 Bean 缓存
    singletons: RwLock<HashMap<String, Arc<dyn Any + Send + Sync>>>,

    /// 类型到名称的映射
    type_to_name: RwLock<HashMap<TypeId, String>>,

    /// 循环依赖检测
    creation_tracker: CreationTracker,

    /// Bean 后置处理器列表（按优先级排序）
    bean_post_processors: RwLock<Vec<Arc<dyn BeanPostProcessor>>>,

    /// 配置是否已冻结
    configuration_frozen: RwLock<bool>,
}

impl DefaultListableBeanFactory {
    /// 创建新的 Bean 工厂
    pub fn new() -> Self {
        Self {
            definitions: RwLock::new(HashMap::new()),
            singletons: RwLock::new(HashMap::new()),
            type_to_name: RwLock::new(HashMap::new()),
            creation_tracker: CreationTracker::new(),
            bean_post_processors: RwLock::new(Vec::new()),
            configuration_frozen: RwLock::new(false),
        }
    }

    /// 创建 Bean 实例并调用生命周期回调
    ///
    /// # Spring Bean 生命周期顺序
    /// 1. 实例化（构造函数）
    /// 2. 依赖注入（属性填充）
    /// 3. Aware 接口回调（BeanNameAware, ApplicationContextAware, EnvironmentAware）
    /// 4. BeanPostProcessor.postProcessBeforeInitialization
    /// 5. InitializingBean.afterPropertiesSet - 通过 init callback 实现
    /// 6. 自定义 init-method
    /// 7. BeanPostProcessor.postProcessAfterInitialization
    fn create_bean_internal(&self, name: &str) -> ContainerResult<Arc<dyn Any + Send + Sync>> {
        let definitions = self.definitions.read();

        let definition = definitions
            .get(name)
            .ok_or_else(|| ContainerError::BeanNotFound(name.to_string()))?;

        // 检查循环依赖
        if self
            .creation_tracker
            .is_creating(name)
            .map_err(|e| ContainerError::Other(anyhow::anyhow!("{}", e)))?
        {
            let creating_chain = self
                .creation_tracker
                .current_creating()
                .map_err(|e| ContainerError::Other(anyhow::anyhow!("{}", e)))?;

            return Err(ContainerError::CircularDependency(format!(
                "{} -> {}",
                creating_chain.join(" -> "),
                name
            )));
        }

        // 标记为正在创建
        if !self
            .creation_tracker
            .start_creating(name)
            .map_err(|e| ContainerError::Other(anyhow::anyhow!("{}", e)))?
        {
            return Err(ContainerError::CircularDependency(format!(
                "Detected circular dependency on '{}'",
                name
            )));
        }

        // 使用 RAII 模式确保在任何情况下都会清理标记
        struct CreationGuard<'a> {
            tracker: &'a CreationTracker,
            name: String,
        }

        impl<'a> Drop for CreationGuard<'a> {
            fn drop(&mut self) {
                if let Err(e) = self.tracker.finish_creating(&self.name) {
                    tracing::error!(
                        "Failed to clear creation tracker for '{}': {}",
                        self.name,
                        e
                    );
                }
            }
        }

        let _guard = CreationGuard {
            tracker: &self.creation_tracker,
            name: name.to_string(),
        };

        // 1. 实例化 Bean（构造函数 + 依赖注入）
        let instance = definition.factory.create().map_err(|e| {
            // 保留循环依赖错误，不要包装它
            match e {
                ContainerError::CircularDependency(_) => e,
                _ => ContainerError::BeanCreationFailed(format!("{}: {}", name, e)),
            }
        })?;

        let mut bean: Arc<dyn Any + Send + Sync> = Arc::from(instance);

        // 2. BeanPostProcessor.postProcessBeforeInitialization
        bean = self.apply_bean_post_processors_before_initialization(bean, name)?;

        // 3. InitializingBean.afterPropertiesSet + 自定义 init-method
        // 通过 init callback 统一处理
        if let Some(ref init_fn) = definition.init_callback {
            if let Some(bean_mut) = Arc::get_mut(&mut bean) {
                init_fn(bean_mut).map_err(|e| {
                    ContainerError::BeanCreationFailed(format!("{} init failed: {}", name, e))
                })?;
            } else {
                tracing::warn!("Cannot call init on bean '{}': multiple references exist", name);
            }
        }

        // 4. BeanPostProcessor.postProcessAfterInitialization
        bean = self.apply_bean_post_processors_after_initialization(bean, name)?;

        Ok(bean)
    }

    /// 应用 BeanPostProcessor.postProcessBeforeInitialization
    fn apply_bean_post_processors_before_initialization(
        &self,
        bean: Arc<dyn Any + Send + Sync>,
        bean_name: &str,
    ) -> ContainerResult<Arc<dyn Any + Send + Sync>> {
        let processors = self.bean_post_processors.read();
        let mut current_bean = bean;

        for processor in processors.iter() {
            current_bean = processor.post_process_before_initialization(current_bean, bean_name)?;
        }

        Ok(current_bean)
    }

    /// 应用 BeanPostProcessor.postProcessAfterInitialization
    fn apply_bean_post_processors_after_initialization(
        &self,
        bean: Arc<dyn Any + Send + Sync>,
        bean_name: &str,
    ) -> ContainerResult<Arc<dyn Any + Send + Sync>> {
        let processors = self.bean_post_processors.read();
        let mut current_bean = bean;

        for processor in processors.iter() {
            current_bean = processor.post_process_after_initialization(current_bean, bean_name)?;
        }

        Ok(current_bean)
    }
}

impl Default for DefaultListableBeanFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl BeanFactory for DefaultListableBeanFactory {
    fn get_bean(&self, name: &str) -> ContainerResult<Arc<dyn Any + Send + Sync>> {
        tracing::trace!("Requesting bean: '{}'", name);

        // 先检查定义是否存在
        let scope = {
            let definitions = self.definitions.read();

            let definition = definitions.get(name).ok_or_else(|| {
                tracing::debug!("Bean '{}' not found in container", name);
                ContainerError::BeanNotFound(name.to_string())
            })?;

            definition.scope
        };

        match scope {
            crate::Scope::Singleton => {
                // 检查缓存
                {
                    let singletons = self.singletons.read();

                    if let Some(bean) = singletons.get(name) {
                        tracing::debug!("Returning cached instance of singleton bean '{}'", name);
                        return Ok(Arc::clone(bean));
                    }
                }

                tracing::info!("Creating shared instance of singleton bean '{}'", name);

                // 创建新实例
                let bean = self.create_bean_internal(name)?;

                // 缓存实例
                let mut singletons = self.singletons.write();
                singletons.insert(name.to_string(), Arc::clone(&bean));

                tracing::debug!("Singleton bean '{}' created and cached", name);
                Ok(bean)
            }
            crate::Scope::Prototype => {
                tracing::debug!("Creating new instance of prototype bean '{}'", name);
                // 每次创建新实例
                self.create_bean_internal(name)
            }
        }
    }

    fn contains_bean(&self, name: &str) -> bool {
        self.definitions.read().contains_key(name)
    }
}

impl BeanFactoryExt for DefaultListableBeanFactory {
    fn get_bean_by_type<T: Any + Send + Sync>(&self) -> ContainerResult<Arc<T>> {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        // 首先尝试通过 TypeId 查找
        let name_opt = {
            let type_to_name = self.type_to_name.read();
            type_to_name.get(&type_id).cloned()
        };

        if let Some(name) = name_opt {
            let bean = self.get_bean(&name)?;
            bean.downcast::<T>()
                .map_err(|_| ContainerError::TypeMismatch {
                    expected: type_name.to_string(),
                    found: "unknown".to_string(),
                })
        } else {
            // TypeId查找失败，尝试类型名称匹配
            let name_opt = {
                let definitions = self.definitions.read();

                let mut found_name = None;
                for (name, definition) in definitions.iter() {
                    if definition.factory.type_name() == type_name {
                        found_name = Some(name.clone());
                        break;
                    }
                }
                found_name
            };

            if let Some(name) = name_opt {
                let bean = self.get_bean(&name)?;
                bean.downcast::<T>()
                    .map_err(|_| ContainerError::TypeMismatch {
                        expected: type_name.to_string(),
                        found: "unknown".to_string(),
                    })
            } else {
                Err(ContainerError::BeanNotFound(format!(
                    "No bean found for type '{}'",
                    type_name
                )))
            }
        }
    }

    fn contains_bean_by_type<T: Any + Send + Sync>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        // TypeId 查找
        if self.type_to_name.read().contains_key(&type_id) {
            return true;
        }

        // TypeId 查找失败，尝试类型名称
        let definitions = self.definitions.read();
        for definition in definitions.values() {
            if definition.factory.type_name() == type_name {
                return true;
            }
        }
        false
    }
}

impl ListableBeanFactory for DefaultListableBeanFactory {
    fn get_bean_names(&self) -> Vec<String> {
        self.definitions.read().keys().cloned().collect()
    }

    fn get_bean_names_for_type(&self, type_id: TypeId) -> Vec<String> {
        let definitions = self.definitions.read();
        definitions
            .iter()
            .filter(|(_, def)| def.factory.type_id() == type_id)
            .map(|(name, _)| name.clone())
            .collect()
    }

    fn get_bean_definition_count(&self) -> usize {
        self.definitions.read().len()
    }
}




impl ConfigurableBeanFactory for DefaultListableBeanFactory {
    fn register_bean_definition(&self, name: String, definition: BeanDefinition) -> ContainerResult<()> {
        // 检查配置是否已冻结
        if *self.configuration_frozen.read() {
            return Err(ContainerError::Other(anyhow::anyhow!(
                "Cannot register bean definition: configuration is frozen"
            )));
        }

        let type_id = definition.factory.type_id();
        let type_name = definition.factory.type_name();

        tracing::trace!(
            "Attempting to register bean: name='{}', type='{}', scope={:?}",
            name,
            type_name,
            definition.scope
        );

        // 检查是否已存在
        {
            let definitions = self.definitions.read();
            if definitions.contains_key(&name) {
                tracing::warn!("Bean '{}' already exists, registration failed", name);
                return Err(ContainerError::BeanAlreadyExists(name));
            }
        }

        // 存储定义
        {
            let mut definitions = self.definitions.write();
            definitions.insert(name.clone(), definition);
        }

        // 注册类型到名称的映射
        {
            let mut type_to_name = self.type_to_name.write();
            type_to_name.insert(type_id, name.clone());
        }

        tracing::debug!("Bean definition registered successfully: '{}'", name);
        Ok(())
    }

    fn contains_bean_definition(&self, name: &str) -> bool {
        self.definitions.read().contains_key(name)
    }

    fn remove_bean_definition(&self, name: &str) -> ContainerResult<()> {
        // 检查配置是否已冻结
        if *self.configuration_frozen.read() {
            return Err(ContainerError::Other(anyhow::anyhow!(
                "Cannot remove bean definition: configuration is frozen"
            )));
        }

        let mut definitions = self.definitions.write();
        definitions
            .remove(name)
            .ok_or_else(|| ContainerError::BeanNotFound(name.to_string()))?;

        tracing::debug!("Bean definition removed: '{}'", name);
        Ok(())
    }

    fn add_bean_post_processor(&self, processor: Arc<dyn BeanPostProcessor>) {
        let mut processors = self.bean_post_processors.write();
        processors.push(processor);

        // 按优先级排序（order 值越小优先级越高）
        processors.sort_by_key(|p| p.order());
    }

    fn get_bean_definition(&self, name: &str) -> ContainerResult<BeanDefinition> {
        let definitions = self.definitions.read();
        definitions
            .get(name)
            .cloned()
            .ok_or_else(|| ContainerError::BeanNotFound(name.to_string()))
    }

    fn modify_bean_definition<F>(&self, name: &str, modifier: F) -> ContainerResult<()>
    where
        F: FnOnce(&mut BeanDefinition),
    {
        // 检查配置是否已冻结
        if *self.configuration_frozen.read() {
            return Err(ContainerError::Other(anyhow::anyhow!(
                "Cannot modify bean definition: configuration is frozen"
            )));
        }

        let mut definitions = self.definitions.write();
        if let Some(definition) = definitions.get_mut(name) {
            modifier(definition);
            tracing::debug!("Bean definition '{}' modified successfully", name);
            Ok(())
        } else {
            Err(ContainerError::BeanNotFound(name.to_string()))
        }
    }

    fn get_bean_post_processors(&self) -> Vec<Arc<dyn BeanPostProcessor>> {
        self.bean_post_processors.read().clone()
    }
}

impl ConfigurableListableBeanFactory for DefaultListableBeanFactory {
    fn preinstantiate_singletons(&self) -> ContainerResult<()> {
        let bean_names: Vec<String> = {
            let definitions = self.definitions.read();
            definitions
                .iter()
                .filter(|(_, def)| def.scope == crate::Scope::Singleton && !def.lazy)
                .map(|(name, _)| name.clone())
                .collect()
        };

        tracing::debug!("Pre-instantiating {} singleton beans", bean_names.len());

        for name in bean_names {
            tracing::debug!("Creating shared instance of singleton bean '{}'", name);
            self.get_bean(&name)?;
        }

        Ok(())
    }

    fn freeze_configuration(&self) {
        let mut frozen = self.configuration_frozen.write();
        *frozen = true;
        tracing::debug!("Bean factory configuration frozen");
    }

    fn is_configuration_frozen(&self) -> bool {
        *self.configuration_frozen.read()
    }

    fn destroy_singletons(&self) -> ContainerResult<()> {
        tracing::info!("Destroying singleton beans");

        // 获取所有定义的克隆，避免长时间持锁
        let definitions_map: std::collections::HashMap<String, bool> = {
            let definitions = self.definitions.read();

            definitions
                .iter()
                .map(|(name, def)| (name.clone(), def.destroy_callback.is_some()))
                .collect()
        };

        // 移除所有单例并尝试调用 destroy
        let mut singletons = self.singletons.write();

        let beans_to_destroy: Vec<(String, Arc<dyn Any + Send + Sync>)> =
            singletons.drain().collect();

        drop(singletons); // 释放写锁

        // 尝试对每个 Bean 调用 destroy
        for (name, mut bean) in beans_to_destroy {
            if definitions_map.get(&name).copied().unwrap_or(false) {
                // 有 destroy 回调
                let definitions = self.definitions.read();

                if let Some(definition) = definitions.get(&name) {
                    if let Some(ref destroy_fn) = definition.destroy_callback {
                        // 尝试获取可变引用（只有引用计数为1时才能成功）
                        if let Some(bean_mut) = Arc::get_mut(&mut bean) {
                            destroy_fn(bean_mut).map_err(|e| {
                                tracing::warn!("Failed to destroy bean '{}': {}", name, e);
                                e
                            })?;
                            tracing::debug!("Bean '{}' destroyed successfully", name);
                        } else {
                            tracing::warn!(
                                "Cannot destroy bean '{}': still has active references",
                                name
                            );
                        }
                    }
                }
            }
        }

        tracing::info!("Singleton beans destruction completed");
        Ok(())
    }

    fn get_bean_definitions(&self) -> std::collections::HashMap<String, Vec<String>> {
        let definitions = self.definitions.read();
        definitions
            .iter()
            .map(|(name, definition)| (name.clone(), definition.dependencies.clone()))
            .collect()
    }
}

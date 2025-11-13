//! 请求/响应拦截器模块
//!
//! 提供类似 Spring Boot HandlerInterceptor 的拦截器功能

use axum::{
    extract::Request,
    http::StatusCode,
    response::Response,
};
use chimera_core::{ApplicationContext, Container};
use async_trait::async_trait;
use std::sync::Arc;

/// 处理器拦截器 trait - 类似Spring的HandlerInterceptor
#[async_trait]
pub trait HandlerInterceptor: Send + Sync {
    fn name(&self) -> &str;
    fn priority(&self) -> i32 {
        100
    }

    /// 请求预处理 - 在控制器方法执行前调用
    /// 返回false表示请求应该被终止
    async fn pre_handle(
        &self,
        request: &mut Request,
        context: &Arc<ApplicationContext>,
    ) -> InterceptorResult<bool>;

    /// 请求后处理 - 在控制器方法执行后调用（但在响应发送前）
    async fn post_handle(
        &self,
        _request: &Request,
        _response: &mut Response,
        _context: &Arc<ApplicationContext>,
    ) -> InterceptorResult<()> {
        // 默认实现：什么都不做
        Ok(())
    }

    /// 完成处理 - 在响应发送后调用（用于清理资源）
    async fn after_completion(
        &self,
        _request: &Request,
        _response: &Response,
        _error: Option<&(dyn std::error::Error + Send + Sync)>,
        _context: &Arc<ApplicationContext>,
    ) -> InterceptorResult<()> {
        // 默认实现：什么都不做
        Ok(())
    }

    /// 拦截器路径匹配 - 决定哪些路径应用此拦截器
    fn path_patterns(&self) -> Vec<&str> {
        vec!["/**"] // 默认拦截所有路径
    }

    /// 排除路径 - 不应用此拦截器的路径
    fn exclude_patterns(&self) -> Vec<&str> {
        vec![] // 默认不排除任何路径
    }
}

/// 拦截器执行结果
pub type InterceptorResult<T> = Result<T, InterceptorError>;

#[derive(Debug, thiserror::Error)]
pub enum InterceptorError {
    #[error("Interceptor execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Authentication required")]
    AuthenticationRequired,

    #[error("Access denied")]
    AccessDenied,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Request validation failed: {0}")]
    ValidationFailed(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
}

impl InterceptorError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::ExecutionFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::AuthenticationRequired => StatusCode::UNAUTHORIZED,
            Self::AccessDenied => StatusCode::FORBIDDEN,
            Self::RateLimitExceeded => StatusCode::TOO_MANY_REQUESTS,
            Self::ValidationFailed(_) => StatusCode::BAD_REQUEST,
            Self::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    pub fn error_message(&self) -> String {
        match self {
            Self::ExecutionFailed(msg) => format!("Interceptor execution failed: {}", msg),
            Self::AuthenticationRequired => "Authentication required".to_string(),
            Self::AccessDenied => "Access denied".to_string(),
            Self::RateLimitExceeded => "Rate limit exceeded".to_string(),
            Self::ValidationFailed(msg) => format!("Request validation failed: {}", msg),
            Self::ServiceUnavailable(msg) => format!("Service unavailable: {}", msg),
        }
    }
}

/// 简化的路径匹配器
#[derive(Debug, Clone)]
struct PathMatcher {
    patterns: Vec<String>,
}

impl PathMatcher {
    fn new(patterns: Vec<&str>) -> Self {
        Self {
            patterns: patterns.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn matches(&self, path: &str) -> bool {
        if self.patterns.is_empty() {
            return false;
        }

        for pattern in &self.patterns {
            if self.match_pattern(pattern, path) {
                return true;
            }
        }

        false
    }

    fn match_pattern(&self, pattern: &str, path: &str) -> bool {
        // 支持简单的通配符匹配
        if pattern == "/**" {
            return true;
        }

        if pattern.ends_with("/**") {
            let prefix = &pattern[..pattern.len() - 3];
            return path.starts_with(prefix);
        }

        if pattern.contains('*') {
            // 简单的单级通配符支持
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                let prefix = parts[0];
                let suffix = parts[1];
                return path.starts_with(prefix) && path.ends_with(suffix);
            }
        }

        // 精确匹配
        pattern == path
    }
}

/// 拦截器包装器
struct InterceptorWrapper {
    interceptor: Box<dyn HandlerInterceptor>,
    include_matcher: PathMatcher,
    exclude_matcher: PathMatcher,
}

impl InterceptorWrapper {
    fn new(interceptor: Box<dyn HandlerInterceptor>) -> Self {
        let include_matcher = PathMatcher::new(interceptor.path_patterns());
        let exclude_matcher = PathMatcher::new(interceptor.exclude_patterns());

        Self {
            interceptor,
            include_matcher,
            exclude_matcher,
        }
    }

    fn should_apply(&self, path: &str) -> bool {
        let included = self.include_matcher.matches(path);
        let excluded = self.exclude_matcher.matches(path);
        included && !excluded
    }
}

/// 拦截器注册表
pub struct InterceptorRegistry {
    interceptors: Vec<InterceptorWrapper>,
}

impl InterceptorRegistry {
    pub fn new() -> Self {
        Self {
            interceptors: Vec::new(),
        }
    }

    pub fn register<I: HandlerInterceptor + 'static>(&mut self, interceptor: I) {
        self.interceptors.push(InterceptorWrapper::new(Box::new(interceptor)));

        // 按优先级排序
        self.interceptors
            .sort_by_key(|w| w.interceptor.priority());
    }

    pub fn register_boxed(&mut self, interceptor: Box<dyn HandlerInterceptor>) {
        self.interceptors.push(InterceptorWrapper::new(interceptor));
        // 按优先级排序
        self.interceptors
            .sort_by_key(|w| w.interceptor.priority());
    }

    pub async fn pre_handle(
        &self,
        request: &mut Request,
        context: &Arc<ApplicationContext>,
    ) -> InterceptorResult<bool> {
        let path = request.uri().path().to_string();

        for wrapper in &self.interceptors {
            if wrapper.should_apply(&path) {
                let continue_processing = wrapper
                    .interceptor
                    .pre_handle(request, context)
                    .await
                    .map_err(|e| {
                        tracing::warn!(
                            interceptor = wrapper.interceptor.name(),
                            error = %e,
                            path = &path,
                            "Interceptor pre_handle failed"
                        );
                        e
                    })?;

                if !continue_processing {
                    tracing::info!(
                        interceptor = wrapper.interceptor.name(),
                        path = &path,
                        "Request terminated by interceptor"
                    );
                    return Ok(false); // 请求被某个拦截器终止
                }
            }
        }

        Ok(true)
    }

    pub async fn post_handle(
        &self,
        request: &Request,
        response: &mut Response,
        context: &Arc<ApplicationContext>,
    ) -> InterceptorResult<()> {
        let path = request.uri().path().to_string();

        // 反向执行post_handle（LIFO顺序）
        for wrapper in self.interceptors.iter().rev() {
            if wrapper.should_apply(&path) {
                wrapper
                    .interceptor
                    .post_handle(request, response, context)
                    .await
                    .map_err(|e| {
                        tracing::warn!(
                            interceptor = wrapper.interceptor.name(),
                            error = %e,
                            path = &path,
                            "Interceptor post_handle failed"
                        );
                        e
                    })?;
            }
        }

        Ok(())
    }

    pub async fn after_completion(
        &self,
        request: &Request,
        response: &Response,
        error: Option<&(dyn std::error::Error + Send + Sync)>,
        context: &Arc<ApplicationContext>,
    ) {
        let path = request.uri().path().to_string();

        // 确保所有拦截器的after_completion都被调用，即使有错误
        for wrapper in self.interceptors.iter().rev() {
            if wrapper.should_apply(&path) {
                if let Err(e) = wrapper
                    .interceptor
                    .after_completion(request, response, error, context)
                    .await
                {
                    tracing::error!(
                        interceptor = wrapper.interceptor.name(),
                        error = %e,
                        path = &path,
                        "Interceptor after_completion failed"
                    );
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.interceptors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.interceptors.is_empty()
    }
}

impl Default for InterceptorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 🔥 框架核心：自动发现并构建拦截器注册表
pub async fn build_interceptor_registry(
    _context: &Arc<ApplicationContext>,
) -> chimera_core::ApplicationResult<InterceptorRegistry> {
    // 使用 inventory 机制自动发现所有拦截器
    Ok(crate::interceptor_registry::build_interceptor_registry_from_inventory())
}

/// 🔥 框架扩展接口：允许框架自动注册新的拦截器类型
impl InterceptorRegistry {
    /// 尝试从容器中获取指定类型的拦截器并自动注册
    pub async fn auto_register_type<T>(
        &mut self,
        context: &Arc<ApplicationContext>,
    ) -> chimera_core::ApplicationResult<bool>
    where
        T: HandlerInterceptor + Clone + 'static,
    {
        match context.get_bean_by_type::<T>().await {
            Ok(interceptor) => {
                let interceptor_name = interceptor.name().to_string();
                self.register((*interceptor).clone());
                tracing::info!("✅ Auto-registered interceptor: {}", interceptor_name);
                Ok(true)
            }
            Err(_) => {
                // Bean不存在，这是正常的
                Ok(false)
            }
        }
    }

    /// 批量自动注册多个类型
    /// 这个方法由框架调用，用户也可以在需要时调用
    pub async fn auto_register_common_types(
        &mut self,
        _context: &Arc<ApplicationContext>,
    ) -> chimera_core::ApplicationResult<usize> {
        let initial_count = self.len();

        // 这里可以添加常见的拦截器类型
        // 框架开发者可以在这里添加新的类型，或者用户可以调用auto_register_type

        // 示例：如果用户定义了AuthInterceptor，框架会自动发现
        // self.auto_register_type::<AuthInterceptor>(context).await?;

        let discovered_count = self.len() - initial_count;
        Ok(discovered_count)
    }
}

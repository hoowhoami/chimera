//! 处理器拦截器示例
//!
//! 展示如何实现各种类型的处理器拦截器

use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    response::Response,
};
use chimera_core::ComponentService;
use chimera_core_macros::{bean, Component};
use chimera_web::interceptor::{HandlerInterceptor, InterceptorError, InterceptorResult};
use chimera_web_macros::Interceptor;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Instant,
};

/// JWT认证拦截器 - 类似Spring Security的认证过滤器
/// 🔥 用户只需要添加这两个注解，框架自动完成注册！
#[derive(Interceptor, Component)]
#[bean("authInterceptor")]
pub struct AuthInterceptor {
    #[value("security.jwt.header", default = "Authorization")]
    auth_header: String,

    #[value("security.jwt.prefix", default = "Bearer ")]
    token_prefix: String,

    #[value("security.jwt.secret", default = "default-secret")]
    jwt_secret: String,
}

impl Default for AuthInterceptor {
    fn default() -> Self {
        Self {
            auth_header: "Authorization".to_string(),
            token_prefix: "Bearer ".to_string(),
            jwt_secret: "default-secret".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl HandlerInterceptor for AuthInterceptor {
    fn name(&self) -> &str {
        "AuthInterceptor"
    }

    fn priority(&self) -> i32 {
        100 // 认证应该在大多数拦截器之前执行
    }

    /// 仅拦截需要认证的API路径
    fn path_patterns(&self) -> Vec<&str> {
        vec!["/api/**"]
    }

    /// 排除不需要认证的路径
    fn exclude_patterns(&self) -> Vec<&str> {
        vec![
            "/api/auth/login",
            "/api/auth/register",
            "/api/public/**",
            "/api/health",
        ]
    }

    async fn pre_handle(
        &self,
        request: &mut Request,
        _context: &Arc<chimera_core::ApplicationContext>,
    ) -> InterceptorResult<bool> {
        // 1. 提取JWT token
        let token = self
            .extract_token(request.headers())
            .ok_or(InterceptorError::AuthenticationRequired)?;

        // 2. 验证token（简化实现）
        let claims = self.validate_token(&token).map_err(|e| {
            tracing::warn!("JWT validation failed: {}", e);
            InterceptorError::AuthenticationRequired
        })?;

        // 3. 检查权限
        if !self.check_permissions(&claims, request.uri().path()) {
            return Err(InterceptorError::AccessDenied);
        }

        // 4. 将用户信息注入到请求中（供控制器使用）
        request.extensions_mut().insert(AuthenticatedUser {
            user_id: claims.sub.clone(),
            username: claims.username.clone(),
            roles: claims.roles.clone(),
        });

        tracing::debug!("User {} authenticated successfully", claims.username);
        Ok(true)
    }

    async fn post_handle(
        &self,
        _request: &Request,
        response: &mut Response,
        _context: &Arc<chimera_core::ApplicationContext>,
    ) -> InterceptorResult<()> {
        // 可以在这里添加安全响应头
        response
            .headers_mut()
            .insert("X-Content-Type-Options", "nosniff".parse().unwrap());
        response
            .headers_mut()
            .insert("X-Frame-Options", "DENY".parse().unwrap());

        Ok(())
    }

    async fn after_completion(
        &self,
        request: &Request,
        response: &Response,
        error: Option<&(dyn std::error::Error + Send + Sync)>,
        _context: &Arc<chimera_core::ApplicationContext>,
    ) -> InterceptorResult<()> {
        // 记录访问日志
        let method = request.method().to_string();
        let path = request.uri().path().to_string();
        let status_code = response.status().as_u16();
        let error_msg = error.map(|e| e.to_string());

        if let Some(user) = request.extensions().get::<AuthenticatedUser>() {
            let user_id = user.user_id.clone();
            let username = user.username.clone();

            tracing::info!(
                user_id = %user_id,
                username = %username,
                path = %path,
                method = %method,
                status = %status_code,
                error = ?error_msg,
                "User access logged"
            );
        }

        Ok(())
    }
}

impl AuthInterceptor {
    fn extract_token(&self, headers: &HeaderMap) -> Option<String> {
        headers
            .get(&self.auth_header)?
            .to_str()
            .ok()?
            .strip_prefix(&self.token_prefix)
            .map(|s| s.to_string())
    }

    fn validate_token(&self, token: &str) -> Result<JwtClaims, String> {
        // 简化的JWT验证实现
        // 在实际应用中，应该使用jsonwebtoken库
        if token == "valid-token" {
            Ok(JwtClaims {
                sub: "user123".to_string(),
                username: "testuser".to_string(),
                roles: vec!["USER".to_string()],
                exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
            })
        } else if token == "admin-token" {
            Ok(JwtClaims {
                sub: "admin123".to_string(),
                username: "admin".to_string(),
                roles: vec!["ADMIN".to_string(), "USER".to_string()],
                exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
            })
        } else {
            Err("Invalid token".to_string())
        }
    }

    fn check_permissions(&self, claims: &JwtClaims, path: &str) -> bool {
        // 实现基于角色的访问控制
        if path.starts_with("/api/admin/") {
            return claims.roles.contains(&"ADMIN".to_string());
        }

        if path.starts_with("/api/users/") {
            return claims.roles.contains(&"USER".to_string())
                || claims.roles.contains(&"ADMIN".to_string());
        }

        true // 默认允许访问
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub username: String,
    pub roles: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub username: String,
    pub roles: Vec<String>,
    pub exp: usize,
}

/// 简单的令牌桶限流器
#[derive(Debug)]
struct TokenBucket {
    capacity: u32,
    tokens: u32,
    last_refill: Instant,
    refill_rate: u32, // tokens per second
}

impl TokenBucket {
    fn new(capacity: u32, refill_rate: u32) -> Self {
        Self {
            capacity,
            tokens: capacity,
            last_refill: Instant::now(),
            refill_rate,
        }
    }

    fn try_consume(&mut self) -> bool {
        self.refill();

        if self.tokens > 0 {
            self.tokens -= 1;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);
        let tokens_to_add = (elapsed.as_secs() as u32) * self.refill_rate;

        if tokens_to_add > 0 {
            self.tokens = (self.tokens + tokens_to_add).min(self.capacity);
            self.last_refill = now;
        }
    }
}

/// 限流拦截器
/// 🔥 用户只需要添加这两个注解，框架自动完成注册！
#[derive(Interceptor, Component)]
#[bean("rateLimitInterceptor")]
pub struct RateLimitInterceptor {
    #[value("rate-limit.requests-per-minute", default = "60")]
    requests_per_minute: u32,

    #[value("rate-limit.burst-size", default = "10")]
    burst_size: u32,

    // 使用IP地址作为key的限流器映射
    buckets: Arc<Mutex<HashMap<String, TokenBucket>>>,
}

impl Default for RateLimitInterceptor {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            burst_size: 10,
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl RateLimitInterceptor {
    pub fn new(requests_per_minute: u32, burst_size: u32) -> Self {
        Self {
            requests_per_minute,
            burst_size,
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn get_client_ip(&self, request: &Request) -> String {
        // 尝试从各种头部获取真实IP
        let headers = request.headers();

        if let Some(forwarded) = headers.get("X-Forwarded-For") {
            if let Ok(ip) = forwarded.to_str() {
                if let Some(first_ip) = ip.split(',').next() {
                    return first_ip.trim().to_string();
                }
            }
        }

        if let Some(real_ip) = headers.get("X-Real-IP") {
            if let Ok(ip) = real_ip.to_str() {
                return ip.to_string();
            }
        }

        // 如果无法获取IP，使用默认值
        "unknown".to_string()
    }
}

#[async_trait::async_trait]
impl HandlerInterceptor for RateLimitInterceptor {
    fn name(&self) -> &str {
        "RateLimitInterceptor"
    }

    fn priority(&self) -> i32 {
        50 // 在认证之后，业务逻辑之前执行
    }

    fn path_patterns(&self) -> Vec<&str> {
        vec!["/api/**"]
    }

    fn exclude_patterns(&self) -> Vec<&str> {
        vec!["/api/health", "/api/metrics"] // 健康检查不限流
    }

    async fn pre_handle(
        &self,
        request: &mut Request,
        _context: &Arc<chimera_core::ApplicationContext>,
    ) -> InterceptorResult<bool> {
        let client_ip = self.get_client_ip(request);
        let mut buckets = self.buckets.lock().unwrap();

        // 获取或创建该IP的令牌桶
        let bucket = buckets.entry(client_ip.clone()).or_insert_with(|| {
            TokenBucket::new(
                self.burst_size,
                self.requests_per_minute / 60, // 转换为每秒
            )
        });

        // 尝试消费一个令牌
        if bucket.try_consume() {
            tracing::debug!("Rate limit check passed for IP: {}", client_ip);
            Ok(true)
        } else {
            tracing::warn!("Rate limit exceeded for IP: {}", client_ip);
            Err(InterceptorError::RateLimitExceeded)
        }
    }

    async fn post_handle(
        &self,
        _request: &Request,
        response: &mut Response,
        _context: &Arc<chimera_core::ApplicationContext>,
    ) -> InterceptorResult<()> {
        // 添加限流相关的响应头
        response.headers_mut().insert(
            "X-RateLimit-Limit",
            self.requests_per_minute.to_string().parse().unwrap(),
        );

        // 可以添加剩余请求数等信息
        Ok(())
    }
}

/// CORS拦截器 - 处理跨域请求
/// 🔥 用户只需要添加这两个注解，框架自动完成注册！
#[derive(Interceptor, Component)]
#[bean("corsInterceptor")]
pub struct CorsInterceptor {
    #[value("cors.allowed-origins", default = "*")]
    allowed_origins: String,

    #[value("cors.allowed-methods", default = "GET,POST,PUT,DELETE,OPTIONS")]
    allowed_methods: String,

    #[value("cors.allowed-headers", default = "Content-Type,Authorization")]
    allowed_headers: String,
}

impl Default for CorsInterceptor {
    fn default() -> Self {
        Self {
            allowed_origins: "*".to_string(),
            allowed_methods: "GET,POST,PUT,DELETE,OPTIONS".to_string(),
            allowed_headers: "Content-Type,Authorization".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl HandlerInterceptor for CorsInterceptor {
    fn name(&self) -> &str {
        "CorsInterceptor"
    }

    fn priority(&self) -> i32 {
        10 // 很高优先级，需要最早处理CORS
    }

    async fn pre_handle(
        &self,
        request: &mut Request,
        _context: &Arc<chimera_core::ApplicationContext>,
    ) -> InterceptorResult<bool> {
        // 对于OPTIONS请求（预检请求），直接返回false来终止处理
        if request.method() == "OPTIONS" {
            tracing::debug!("CORS preflight request detected");
            return Ok(false); // 会在post_handle中设置CORS头并返回
        }

        Ok(true)
    }

    async fn post_handle(
        &self,
        request: &Request,
        response: &mut Response,
        _context: &Arc<chimera_core::ApplicationContext>,
    ) -> InterceptorResult<()> {
        let headers = response.headers_mut();

        // 设置CORS头
        headers.insert(
            "Access-Control-Allow-Origin",
            self.allowed_origins.parse().unwrap(),
        );

        headers.insert(
            "Access-Control-Allow-Methods",
            self.allowed_methods.parse().unwrap(),
        );

        headers.insert(
            "Access-Control-Allow-Headers",
            self.allowed_headers.parse().unwrap(),
        );

        headers.insert("Access-Control-Max-Age", "86400".parse().unwrap());

        // 对于OPTIONS请求，设置204状态码
        let method = request.method().to_string();
        if method == "OPTIONS" {
            *response.status_mut() = StatusCode::NO_CONTENT;
        }

        Ok(())
    }
}

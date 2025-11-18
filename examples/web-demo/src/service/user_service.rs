use chimera_core::prelude::*;
use chimera_core_macros::Component;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::models::{User, CreateUserRequest, UpdateUserRequest, SearchQuery};

// 简单的错误类型（用于 AOP 演示）
#[derive(Debug, Clone)]
pub struct ServiceError(pub String);

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ServiceError {}

#[derive(Component, Clone)]
#[bean("userService")]
pub struct UserService {
    #[autowired]
    _config: Arc<AppConfig>,
}

impl UserService {
    // ============================================================================
    // 原有方法（不使用 AOP）
    // ============================================================================

    pub fn list_users(&self) -> Vec<User> {
        vec![
            User {
                id: 1,
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
            },
            User {
                id: 2,
                name: "Bob".to_string(),
                email: "bob@example.com".to_string(),
            },
        ]
    }

    pub fn get_user_by_id(&self, id: u32) -> Option<User> {
        self.list_users().into_iter().find(|u| u.id == id)
    }

    pub fn create_user(&self, request: CreateUserRequest) -> User {
        User {
            id: 100,
            name: request.name,
            email: request.email,
        }
    }

    pub fn update_user(&self, id: u32, request: UpdateUserRequest) -> Option<User> {
        Some(User {
            id,
            name: request.name.unwrap_or_else(|| "Updated User".to_string()),
            email: request.email.unwrap_or_else(|| "updated@example.com".to_string()),
        })
    }

    pub fn search_users(&self, query: SearchQuery) -> Vec<User> {
        let mut users = self.list_users();

        if let Some(name) = query.name {
            users.retain(|u| u.name.contains(&name));
        }
        if let Some(email) = query.email {
            users.retain(|u| u.email.contains(&email));
        }

        users
    }

    // ============================================================================
    // 使用 AOP 的演示方法 - 纯粹的业务逻辑（无需任何 AOP 注解）
    // ============================================================================

    /// 获取用户 - 纯粹的业务逻辑，AOP 通过切点表达式自动应用
    pub async fn get_user_with_aop(&self, id: u32) -> Result<User, ServiceError> {
        // 模拟数据库查询延迟
        std::thread::sleep(std::time::Duration::from_millis(30));

        self.get_user_by_id(id)
            .ok_or_else(|| ServiceError(format!("User {} not found", id)))
    }

    /// 创建用户 - 纯粹的业务逻辑，AOP 自动应用事务切面
    pub async fn create_user_with_aop(&self, request: CreateUserRequest) -> Result<User, ServiceError> {
        // 模拟数据库写入延迟
        std::thread::sleep(std::time::Duration::from_millis(80));

        Ok(self.create_user(request))
    }

    /// 更新用户 - 纯粹的业务逻辑
    pub async fn update_user_with_aop(&self, id: u32, request: UpdateUserRequest) -> Result<User, ServiceError> {
        // 模拟数据库更新延迟
        std::thread::sleep(std::time::Duration::from_millis(60));

        self.update_user(id, request)
            .ok_or_else(|| ServiceError(format!("Failed to update user {}", id)))
    }

    /// 慢查询 - 纯粹的业务逻辑，AOP 自动监控性能
    pub async fn slow_query_with_aop(&self) -> Result<Vec<User>, ServiceError> {
        // 模拟慢查询（超过性能阈值）
        std::thread::sleep(std::time::Duration::from_millis(100));

        Ok(self.list_users())
    }

    /// 错误方法 - 纯粹的业务逻辑，AOP 自动处理异常
    pub async fn error_method_with_aop(&self) -> Result<User, ServiceError> {
        Err(ServiceError("Simulated database connection error".to_string()))
    }
}

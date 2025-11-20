use chimera_core::prelude::*;
use chimera_core_macros::Component;
use chimera_web::prelude::*;
use chimera_web_macros::{controller, get_mapping, request_mapping};
use serde_json::json;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::service::UserService;

#[controller("/api")]
#[derive(Component, Clone)]
pub struct ApiController {
    #[autowired]
    user_service: Arc<UserService>,

    #[autowired]
    config: Arc<AppConfig>,
}

#[controller]
impl ApiController {
    // ========== 无参数方法 ==========

    /// GET /api/info
    #[get_mapping("/info")]
    async fn get_info(&self) -> impl IntoResponse {
        ResponseEntity::ok(json!({
            "app": self.config.name,
            "version": self.config.version,
            "status": "running"
        }))
    }

    #[request_mapping("/health")]
    async fn health_check(&self) -> impl IntoResponse {
        ResponseEntity::ok(json!({
            "status": "healthy",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}

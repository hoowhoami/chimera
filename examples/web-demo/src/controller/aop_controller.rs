use chimera_core::prelude::*;
use chimera_core_macros::Component;
use chimera_web_macros::{Controller, controller, get_mapping, post_mapping, put_mapping};
use chimera_web::prelude::*;
use chimera_web::extractors::{PathVariable, RequestBody};
use serde_json::json;
use std::sync::Arc;

use crate::service::UserService;
use crate::models::{CreateUserRequest, UpdateUserRequest};

/// AOP 演示 Controller
///
/// 提供 AOP（面向切面编程）功能演示端点
#[derive(Controller, Component, Clone)]
#[route("/aop")]
pub struct AopController {
    #[autowired]
    user_service: Arc<UserService>,
}

#[controller]
impl AopController {

    /// AOP 初始化说明
    #[get_mapping("/init")]
    async fn init_aop(&self) -> impl IntoResponse {
        ResponseEntity::ok(json!({
            "message": "AOP 初始化说明",
            "note": "在实际应用中，AOP 应该在应用启动时初始化。这里仅作演示。",
            "usage": "调用 /api/aop/demo/* 端点来查看 AOP 效果"
        }))
    }


    /// 演示 AOP - 获取用户
    #[get_mapping("/get-user/:id")]
    async fn demo_aop_get_user(&self, PathVariable(id): PathVariable<u32>) -> impl IntoResponse {
        match self.user_service.get_user_with_aop(id).await {
            Ok(user) => ResponseEntity::ok(json!({
                "message": "成功获取用户（使用 AOP）",
                "data": user,
                "note": "查看日志输出，可以看到 AOP 切面的执行记录"
            })),
            Err(e) => ResponseEntity::not_found(json!({
                "error": e.to_string(),
                "note": "查看日志输出，可以看到异常切面的记录"
            }))
        }
    }


    /// 演示 AOP - 创建用户
    #[post_mapping("/create-user")]
    async fn demo_aop_create_user(&self, RequestBody(req): RequestBody<CreateUserRequest>) -> impl IntoResponse {
        match self.user_service.create_user_with_aop(req).await {
            Ok(user) => ResponseEntity::ok(json!({
                "message": "成功创建用户（使用 AOP）",
                "data": user,
                "note": "查看日志输出，可以看到：\n- 事务开始\n- 方法执行日志\n- 事务提交"
            })),
            Err(e) => ResponseEntity::internal_error(json!({ "error": e.to_string() }))
        }
    }


    /// 演示 AOP - 更新用户
    #[put_mapping("/update-user/:id")]
    async fn demo_aop_update_user(
        &self,
        PathVariable(id): PathVariable<u32>,
        RequestBody(req): RequestBody<UpdateUserRequest>
    ) -> impl IntoResponse {
        match self.user_service.update_user_with_aop(id, req).await {
            Ok(user) => ResponseEntity::ok(json!({
                "message": "成功更新用户（使用 AOP）",
                "data": user,
                "note": "查看日志输出，可以看到事务和性能监控切面的记录"
            })),
            Err(e) => ResponseEntity::internal_error(json!({ "error": e.to_string() }))
        }
    }


    /// 演示 AOP - 慢查询性能监控
    #[get_mapping("/slow-query")]
    async fn demo_aop_slow_query(&self) -> impl IntoResponse {
        match self.user_service.slow_query_with_aop().await {
            Ok(users) => ResponseEntity::ok(json!({
                "message": "慢查询执行完成（使用 AOP）",
                "data": users,
                "note": "查看日志输出，可以看到性能监控切面发出的警告（超过阈值50ms）"
            })),
            Err(e) => ResponseEntity::internal_error(json!({ "error": e.to_string() }))
        }
    }


    /// 演示 AOP - 异常处理
    #[get_mapping("/error")]
    async fn demo_aop_error(&self) -> impl IntoResponse {
        match self.user_service.error_method_with_aop().await {
            Ok(_) => ResponseEntity::ok(json!({ "message": "不应该到这里" })),
            Err(e) => ResponseEntity::internal_error(json!({
                "error": e.to_string(),
                "note": "查看日志输出，可以看到：\n- 异常切面记录错误\n- 事务回滚日志"
            }))
        }
    }
}

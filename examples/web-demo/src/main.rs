use chimera_core::prelude::*;

// 导入模块
mod config;
mod models;
mod service;
mod controller;
mod error;
mod aspects;      // AOP 切面模块
mod processors;   // BeanPostProcessor 示例模块

// 导入异常处理器模块
mod handlers {
    pub mod exception_handlers;
}

// ==================== 主程序 ====================

#[tokio::main]
async fn main() -> ApplicationResult<()> {
    // 配置文件会自动从以下位置查找（按优先级）：
    // 1. config/application.toml
    // 2. application.toml
    // 也可以手动指定：.config_file("custom/path/to/config.toml")

    // 一行启动应用并阻塞（类似 Spring Boot 的 SpringApplication.run()）
    ChimeraApplication::new("WebDemo")
        .env_prefix("WEB_")
        .run_until_shutdown()
        .await
}

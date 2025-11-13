use chimera_core::prelude::*;
use chimera_core_macros::ConfigurationProperties;

// ==================== 配置 ====================

#[derive(ConfigurationProperties, Debug, Clone)]
#[prefix("app")]
struct AppConfig {
    name: String,
    version: String,
}

// ==================== 主程序 ====================

#[tokio::main]
async fn main() -> ApplicationResult<()> {
    println!("🌐 Chimera Web - 自动装配演示\n");

    // 配置文件路径
    let config_file = if std::path::Path::new("examples/web-demo/application.toml").exists() {
        "examples/web-demo/application.toml"
    } else {
        "application.toml"
    };

    // ✨ 只需要一行启动代码！
    // Web 服务器自动配置和启动，无需任何手动配置
    ChimeraApplication::new("WebDemo")
        .config_file(config_file)
        .env_prefix("WEB_")
        .run()
        .await?
        .wait_for_shutdown()
        .await?;

    Ok(())
}

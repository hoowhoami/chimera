# Chimera

一个受 Spring Boot 启发的 Rust 依赖注入框架

## 特性

### chimera-core

核心依赖注入容器，提供以下功能：

- ✅ **类型安全的依赖注入**：基于 Rust 的类型系统，编译时检查
- ✅ **多种作用域**：支持单例（Singleton）和原型（Prototype）
- ✅ **灵活的注册方式**：支持函数工厂、构建器模式
- ✅ **线程安全**：使用 `Arc` 和 `RwLock` 保证并发安全
- ✅ **延迟初始化**：支持懒加载单例 Bean
- ✅ **生命周期管理**：自动管理 Bean 的创建和销毁
- ✅ **自动装配**：通过宏实现类似 Spring Boot 的自动装配
- ✅ **依赖验证**：静态检测循环依赖和缺失依赖
- ✅ **配置管理**：支持 TOML/环境变量等多种配置源，优先级管理
- ✅ **Profile 支持**：类似 Spring 的 dev/prod 环境配置
- ✅ **@Value 注入**：直接从配置注入值到字段
- ✅ **@ConfigurationProperties**：自动批量绑定配置，注册为 Bean，支持依赖注入
- ✅ **应用启动器**：Spring Boot 风格的 ChimeraApplication.run() 启动方式
- ✅ **日志模块**：内置日志配置，支持多种格式和级别，自动初始化

## 快速开始

### 查看示例

```bash
# 运行综合示例（推荐）- 展示所有核心特性
cargo run -p app-demo

# 测试环境变量覆盖
APP_SERVER_PORT=9000 cargo run -p app-demo
```

更多示例请查看 [examples/README.md](examples/README.md)

### 添加依赖

```toml
[dependencies]
chimera-core = { path = "chimera-core" }
chimera-macros = { path = "chimera-macros" }
inventory = "0.3"  # 自动组件扫描需要
```

### 基本使用

```rust
use chimera_core::prelude::*;
use chimera_macros::{Component, ConfigurationProperties};
use std::sync::Arc;

// 定义配置
#[derive(ConfigurationProperties, Debug, Clone)]
#[prefix("database")]
struct DatabaseConfig {
    host: String,
    port: i32,
}

// 定义服务
#[derive(Component)]
struct DatabaseService {
    #[autowired]
    config: Arc<DatabaseConfig>,
}

fn main() -> ApplicationResult<()> {
    // 一行启动，全自动配置
    let context = ChimeraApplication::new("MyApp")
        .config_file("application.toml")
        .run()?;

    // 获取服务
    let service = context.get_bean_by_type::<DatabaseService>()?;

    Ok(())
}
```

更完整的示例请运行：`cargo run -p app-demo`

## ChimeraApplication - Spring Boot 风格启动

Chimera 提供类似 `SpringApplication.run()` 的启动方式，自动完成配置加载、组件扫描、依赖验证等步骤。

### 基本使用

```rust
use chimera_core::prelude::*;
use chimera_macros::Component;

fn main() -> ApplicationResult<()> {
    // 🚀 一行启动应用 - 自动加载配置、扫描组件、验证依赖
    let context = ChimeraApplication::new("MyApp")
        .run()?;

    // 使用应用上下文
    let service = context.get_bean_by_type::<MyService>()?;
    service.start();

    Ok(())
}
```

### 完整配置示例

```rust
use chimera_core::prelude::*;
use chimera_macros::Component;
use std::sync::Arc;

// 配置服务
#[derive(Debug, Clone)]
struct AppConfig {
    env: Arc<Environment>,
}

impl AppConfig {
    fn new(env: Arc<Environment>) -> Self {
        Self { env }
    }

    fn app_name(&self) -> String {
        self.env.get_string_or("app.name", "MyApp")
    }

    fn server_port(&self) -> i64 {
        self.env.get_i64_or("server.port", 8080)
    }
}

// 业务服务（自动注入配置）
#[derive(Component)]
struct ServerService {
    #[autowired]
    config: Arc<AppConfig>,
}

fn main() -> ApplicationResult<()> {
    let context = ChimeraApplication::new("MyApplication")
        .config_file("application.toml")        // 配置文件路径
        .env_prefix("APP_")                     // 环境变量前缀
        .profiles(vec!["dev".to_string()])      // 激活的 profiles
        .banner(true)                           // 显示 banner
        .initializer(|ctx| {                    // 自定义初始化器
            // 注册配置服务
            let env = Arc::clone(ctx.environment());
            ctx.register_singleton("appConfig", move || {
                Ok(AppConfig::new(Arc::clone(&env)))
            })?;
            Ok(())
        })
        .run()?;

    // 获取并使用服务
    let server = context.get_bean_by_type::<ServerService>()?;
    println!("🚀 Starting on port: {}", server.config.server_port());

    Ok(())
}
```

### 配置文件 (application.toml)

```toml
[app]
name = "MyApplication"
version = "1.0.0"

[server]
host = "0.0.0.0"
port = 8080

[database]
url = "postgres://localhost:5432/myapp"
pool_size = 10
```

### 环境变量覆盖

```bash
# 环境变量会覆盖配置文件中的值
APP_SERVER_PORT=9000 APP_DATABASE_URL=mysql://custom cargo run

# 启动时会看到：
#   ____ _     _
#  / ___| |__ (_)_ __ ___   ___ _ __ __ _
# | |   | '_ \| | '_ ` _ \ / _ \ '__/ _` |
# | |___| | | | | | | | | |  __/ | | (_| |
#  \____|_| |_|_|_| |_| |_|\___|_|  \__,_|
#
#  :: Chimera Framework ::        (v0.1.0)
#
# INFO Starting MyApplication application
# INFO Loaded configuration from: application.toml
# INFO ApplicationContext initialized
# INFO Scanning for @Component annotated beans
# INFO Validating bean dependencies
# 🚀 Starting on port: 9000  ← 环境变量生效
```

### ChimeraApplication API

| 方法 | 说明 |
|------|------|
| `new(name)` | 创建应用，指定名称 |
| `config_file(path)` | 设置配置文件路径（默认 `application.toml`） |
| `config_files(paths)` | 添加多个配置文件 |
| `env_prefix(prefix)` | 设置环境变量前缀（默认 `APP_`） |
| `profiles(profiles)` | 设置激活的 profiles |
| `banner(show)` | 是否显示 banner（默认 true） |
| `initializer(fn)` | 添加初始化器（在组件扫描前执行） |
| `run()` | 启动应用并返回 ApplicationContext |

### 启动流程

`ChimeraApplication.run()` 执行以下步骤：

1. ✅ 显示 banner（如果启用）
2. ✅ 加载配置文件（TOML）
3. ✅ 添加环境变量配置源
4. ✅ 设置 profiles
5. ✅ 构建 ApplicationContext
6. ✅ 执行自定义初始化器
7. ✅ 自动扫描组件（`@Component`）
8. ✅ 验证依赖关系
9. ✅ 返回可用的 ApplicationContext

运行 `cargo run -p app-demo` 查看完整示例。

### 依赖注入示例

```rust
#[derive(Debug)]
struct UserRepository {
    db: Arc<DatabaseService>,
}

impl UserRepository {
    fn new(db: Arc<DatabaseService>) -> Self {
        Self { db }
    }
}

// 注册带依赖的 Bean
let db = context.get_bean_by_type::<DatabaseService>()?;
context.register_singleton("user_repository", move || {
    Ok(UserRepository::new(Arc::clone(&db)))
})?;
```

## Bean 作用域

### 单例（Singleton）

默认作用域，容器中只有一个实例：

```rust
context.register_singleton("config", || {
    Ok(ConfigService::new())
})?;
```

### 原型（Prototype）

每次获取都创建新实例：

```rust
context.register_prototype("request", || {
    Ok(RequestContext::new())
})?;
```

## 运行示例

```bash
# 运行测试
cargo test -p chimera-core

# 运行综合示例（推荐）- 展示所有核心特性
cargo run -p app-demo

# 测试环境变量覆盖
APP_SERVER_PORT=9000 cargo run -p app-demo
APP_DATABASE_HOST=prod-db cargo run -p app-demo

# 运行配置绑定示例 - 深入了解 @ConfigurationProperties
cargo run -p config-properties-demo

# 测试配置覆盖
APP_DATABASE_HOST=prod-db APP_SERVER_PORT=9000 cargo run -p config-properties-demo
```

## 自动装配

Chimera 支持通过宏实现类似 Spring Boot 的自动装配功能，无需手动注册每个组件。

### 使用 #[derive(Component)] 宏

```rust
use chimera_core::prelude::*;
use chimera_macros::Component;
use std::sync::Arc;

// 基础服务
struct ConfigService {
    app_name: String,
}

// 使用 Component 宏自动实现依赖注入
#[derive(Component)]
#[bean("database")]        // 指定 bean 名称
#[scope("singleton")]      // 指定作用域
struct DatabaseService {
    #[autowired]           // 自动注入依赖
    config: Arc<ConfigService>,
}

#[derive(Component)]
#[bean("userService")]
struct UserService {
    #[autowired]
    db: Arc<DatabaseService>,
    #[autowired]
    config: Arc<ConfigService>,
}

fn main() -> Result<()> {
    let context = Arc::new(ApplicationContext::new());

    // 手动注册基础服务
    context.register_singleton("config", || {
        Ok(ConfigService {
            app_name: "MyApp".to_string(),
        })
    })?;

    // 🎯 自动扫描并注册所有Component - 无需手动逐个注册！
    context.scan_components()?;

    // 获取并使用服务
    let service = context.get_bean_by_type::<UserService>()?;

    Ok(())
}
```

### 支持的属性

- `#[bean("name")]` - 指定 Bean 名称（可选，默认为类型名的小驼峰形式）
- `#[scope("singleton")]` 或 `#[scope("prototype")]` - 指定作用域（可选，默认为 singleton）
- `#[lazy]` - 标记为延迟初始化（可选）
- `#[autowired]` - 标记字段需要自动注入（必须是 `Arc<T>` 类型）
- `#[value("config.key")]` - 从配置中注入值（支持 String、i64、f64、bool 等类型）
- `#[value("config.key", default = value)]` - 从配置中注入值，带默认值

### @Value 配置注入

使用 `#[value]` 属性可以直接将配置值注入到字段中：

```rust
use chimera_core::prelude::*;
use chimera_macros::Component;

#[derive(Component, Debug, Clone)]
struct AppConfig {
    // 必需配置 - 如果不存在会报错
    #[value("app.name")]
    app_name: String,

    // 可选配置 - 带默认值
    #[value("app.version", default = "1.0.0")]
    version: String,

    #[value("app.debug", default = false)]
    debug: bool,

    #[value("server.port", default = 8080)]
    port: i64,

    #[value("database.timeout", default = 30.0)]
    timeout: f64,
}

fn main() -> ApplicationResult<()> {
    let context = ChimeraApplication::new("MyApp")
        .config_file("application.toml")
        .env_prefix("APP_")
        .run()?;

    let config = context.get_bean_by_type::<AppConfig>()?;
    println!("App: {} v{}", config.app_name, config.version);
    println!("Port: {}, Debug: {}", config.port, config.debug);

    Ok(())
}
```

**支持的类型**：
- ✅ `String` - 字符串值
- ✅ `i64`, `i32`, `u64`, `u32` - 整数类型
- ✅ `f64`, `f32` - 浮点数类型
- ✅ `bool` - 布尔值（支持 true/false、yes/no、1/0）

**配置来源优先级**（从高到低）：
1. 环境变量（如 `APP_SERVER_PORT`）
2. TOML 配置文件（如 `application.toml`）
3. 默认值（在 `#[value]` 中指定）

运行 `cargo run -p value-injection-demo` 查看完整示例。

### @ConfigurationProperties 批量绑定配置

使用 `#[derive(ConfigurationProperties)]` 宏可以将配置批量绑定到类型安全的结构体，**自动注册为 Bean**，支持依赖注入：

```rust
use chimera_core::prelude::*;
use chimera_macros::{ConfigurationProperties, Component};
use std::sync::Arc;

// 数据库配置 - 自动绑定并注册为 Bean
#[derive(ConfigurationProperties, Debug, Clone)]
#[prefix("database")]  // 配置前缀
struct DatabaseProperties {
    host: String,
    port: i32,
    username: String,
    password: String,

    // 自定义配置键名（kebab-case）
    #[config("max-connections")]
    max_connections: i32,

    timeout: i32,

    // snake_case 自动转换为 kebab-case
    ssl_enabled: bool,  // 对应 database.ssl-enabled
}

// 服务器配置
#[derive(ConfigurationProperties, Debug, Clone)]
#[prefix("server")]
struct ServerProperties {
    host: String,
    port: i32,
    workers: i32,

    #[config("request-timeout")]
    request_timeout: i32,
}

// 业务服务 - 通过 @autowired 自动注入配置
#[derive(Component)]
struct DatabaseService {
    #[autowired]
    config: Arc<DatabaseProperties>,
}

impl DatabaseService {
    fn connect(&self) {
        println!("Connecting to {}:{}", self.config.host, self.config.port);
    }
}

fn main() -> ApplicationResult<()> {
    // ✅ 一行启动 - 自动完成配置绑定和依赖注入
    let context = ChimeraApplication::new("MyApp")
        .config_file("application.toml")
        .env_prefix("APP_")
        .run()?;

    // 方式 1: 从容器获取配置 Bean
    let db_props = context.get_bean_by_type::<DatabaseProperties>()?;
    println!("Database: {}:{}", db_props.host, db_props.port);

    // 方式 2: 使用注入了配置的业务服务
    let db_service = context.get_bean_by_type::<DatabaseService>()?;
    db_service.connect();

    Ok(())
}
```

**配置文件 (application.toml)**：
```toml
[database]
host = "localhost"
port = 5432
username = "postgres"
password = "secret"
max-connections = 20
timeout = 30
ssl-enabled = true

[server]
host = "0.0.0.0"
port = 8080
workers = 4
request-timeout = 60
```

**环境变量覆盖**：
```bash
# 环境变量会自动覆盖配置文件中的值
APP_DATABASE_HOST=prod-db cargo run
APP_SERVER_PORT=9000 cargo run
```

**关键特性**：
- ✅ **自动扫描和绑定** - ChimeraApplication.run() 自动完成，无需手动调用
- ✅ **自动注册为 Bean** - 可通过 `get_bean_by_type()` 获取或 `@autowired` 注入
- ✅ **批量绑定** - 一次绑定所有相关配置，无需逐个 `get_*()`
- ✅ **类型安全** - 编译时检查类型，运行时自动转换
- ✅ **字段名转换** - snake_case 自动转换为 kebab-case
- ✅ **自定义键名** - 支持 `#[config("custom-key")]` 指定配置键
- ✅ **前缀支持** - `#[prefix("database")]` 统一配置前缀
- ✅ **环境变量覆盖** - 保持配置优先级管理
- ✅ **依赖注入** - 可通过 `@autowired` 注入到 Component 中

**Spring Boot 风格的使用体验**：
```rust
// ❌ 传统方式：手动逐个读取配置
let host = env.get_string("database.host")?;
let port = env.get_i64("database.port")? as i32;
let username = env.get_string("database.username")?;
// ... 更多配置

// ✅ 现在：自动绑定 + 自动注册 + 依赖注入
#[derive(ConfigurationProperties, Debug, Clone)]
#[prefix("database")]
struct DatabaseProperties { ... }

// 启动应用即可，配置自动绑定并注册为 Bean
let context = ChimeraApplication::new("MyApp").run()?;

// 使用方式 1: 直接获取
let db_config = context.get_bean_by_type::<DatabaseProperties>()?;

// 使用方式 2: 注入到 Component（推荐）
#[derive(Component)]
struct MyService {
    #[autowired]
    db_config: Arc<DatabaseProperties>,
}
```

运行 `cargo run -p config-properties-demo` 查看完整示例。



### 工作原理

1. `#[derive(Component)]` 宏会自动将组件注册到全局注册表
2. `context.scan_components()` 扫描并注册所有标记的组件
3. 依赖关系会自动解析和注入

### 依赖验证

Chimera 提供静态依赖验证功能，可以在运行前检测潜在问题：

```rust
// 扫描组件后立即验证依赖
context.scan_components()?;
context.validate_dependencies()?;  // 提前发现循环依赖和缺失依赖

// 如果有问题，会返回清晰的错误信息：
// ❌ Circular dependency detected: serviceA -> serviceB -> serviceC -> serviceA
// ❌ Bean 'userService' depends on 'config' which is not registered
```

**验证内容**：
- ✅ 检测循环依赖（A → B → C → A）
- ✅ 检测缺失的依赖（声明了但未注册）
- ✅ 在实际创建 Bean 前发现问题
- ✅ 提供清晰的错误信息和依赖链

运行 `cargo run -p dependency-validation-demo` 查看完整演示。

## 配置管理

Chimera 提供类似 Spring Boot 的配置管理功能：

### Environment - 统一配置访问

```rust
use chimera_core::prelude::*;

// 创建 Environment
let env = Arc::new(Environment::new());

// 添加 TOML 配置源
env.add_property_source(Box::new(
    TomlPropertySource::from_file("application.toml")?
));

// 添加环境变量配置源（优先级更高）
env.add_property_source(Box::new(
    EnvironmentPropertySource::new("APP_")
));

// 读取配置
let app_name = env.get_string_or("app.name", "MyApp");
let port = env.get_i64_or("server.port", 8080);
let enabled = env.get_bool_or("feature.enabled", false);
```

### 配置源优先级

支持多种配置源，按优先级从低到高：

1. **TOML/YAML 文件** (优先级 0)
2. **环境变量** (优先级 100)
3. **运行时配置** (优先级 200)

```rust
// 环境变量会覆盖文件配置
APP_SERVER_PORT=9000 cargo run

// 运行时配置优先级最高
let runtime_config = MapPropertySource::new("runtime")
    .with_property("app.mode", ConfigValue::String("debug".to_string()))
    .with_priority(200);
env.add_property_source(Box::new(runtime_config));
```

### 与 ApplicationContext 集成

```rust
// 通过 Builder 配置 ApplicationContext
let context = ApplicationContext::builder()
    .add_property_source(Box::new(
        TomlPropertySource::from_file("application.toml")?
    ))
    .add_property_source(Box::new(
        EnvironmentPropertySource::new("APP_")
    ))
    .set_active_profiles(vec!["dev".to_string()])
    .build()?;

// 访问 Environment
let env = context.environment();
let db_url = env.get_string("database.url");
```

### Profile 支持

```rust
// 设置激活的 profile
env.set_active_profiles(vec!["dev".to_string(), "local".to_string()]);

// 检查 profile
if env.accepts_profiles("dev") {
    // 开发环境特定逻辑
}
```

运行 `cargo run -p config-demo` 和 `cargo run -p config-integration-demo` 查看完整演示。

### 日志输出

Chimera 使用 `tracing` 框架提供详细的日志输出，类似 Spring Boot 的风格。

```bash
# 运行时查看INFO级别日志
cargo run -p autowiring-demo

# 查看详细的DEBUG级别日志
RUST_LOG=debug cargo run -p autowiring-demo

# 查看更详细的TRACE级别日志
RUST_LOG=trace cargo run -p autowiring-demo
```

## 架构设计

### 核心概念

1. **Container**：依赖注入容器接口
2. **ApplicationContext**：Container 的默认实现
3. **BeanDefinition**：Bean 的定义，包含名称、作用域、工厂等
4. **BeanFactory**：Bean 工厂接口，负责创建 Bean 实例
5. **Scope**：Bean 的作用域（单例、原型）

### 设计原则

- **类型安全**：充分利用 Rust 的类型系统和泛型
- **零成本抽象**：尽可能在编译期完成检查
- **线程安全**：所有 Bean 都是 `Send + Sync`
- **惯用 Rust**：遵循 Rust 的最佳实践和编码规范

## 最佳实践

### 错误处理

使用 `thiserror` 定义错误类型：

```rust
/// 容器级别错误 - 用于 Bean 操作和依赖管理
#[derive(Error, Debug)]
pub enum ContainerError {
    #[error("Bean not found: {0}")]
    BeanNotFound(String),
    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),
    // ...
}

/// 应用级别错误 - 用于应用启动和配置加载
#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("Failed to initialize logger: {0}")]
    LoggingInitFailed(String),
    #[error("Failed to load configuration: {0}")]
    ConfigLoadFailed(String),
    #[error("Container error: {0}")]
    Container(#[from] ContainerError),
    // ...
}
```

**使用场景**：
- `ContainerError` - Bean 操作（注册、查找、创建、依赖注入）
- `ApplicationError` - 应用启动（日志初始化、配置加载、组件扫描）
- `ChimeraApplication.run()` 返回 `ApplicationResult<Arc<ApplicationContext>>`
- 容器方法返回 `Result<T>` (即 `Result<T, ContainerError>`)

### 配置管理

建议使用配置文件（如 TOML、YAML）管理应用配置：

```rust
#[derive(Debug, Deserialize)]
struct AppConfig {
    database: DatabaseConfig,
    server: ServerConfig,
}
```

## 后续规划

- [x] 实现依赖自动装配（通过过程宏 `#[derive(Component)]`）
- [x] 添加生命周期回调（`@PostConstruct`、`@PreDestroy`）
- [x] 实现静态依赖验证（循环依赖和缺失依赖检测）
- [x] 实现配置管理模块（支持 TOML/ENV，优先级管理）
- [x] 实现 ChimeraApplication 启动器（SpringApplication.run() 风格）
- [x] 添加 @Value 宏支持字段注入配置
- [x] 实现 @ConfigurationProperties 批量绑定配置
- [ ] Bean 循环依赖自动解决（通过 Lazy<T> 或 Provider<T>）
- [ ] 支持 Bean 别名和多名称
- [ ] 实现 Bean 事件监听机制
- [ ] 添加 Web 框架集成（Actix-web、Axum）
- [ ] 支持 Bean Profile（开发、测试、生产环境）
- [ ] 实现 Bean 懒加载优化
- [ ] 添加容器启动性能分析工具

## 许可

MIT OR Apache-2.0
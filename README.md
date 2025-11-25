# Chimera

一个受 Spring Boot 启发的 Rust 应用开发框架，提供依赖注入、Web 服务器、配置管理等企业级功能，让您以类型安全、线程安全的方式快速构建 Rust 应用。

## 特点

- **Spring Boot 风格** - 熟悉的注解和开发体验
- **类型安全** - 编译期依赖检查，运行时零成本
- **线程安全** - 所有 Bean 都是 `Send + Sync`
- **Web 框架** - 基于 Axum 的高性能 Web 服务器
- **配置管理** - 多源配置、环境切换、类型绑定
- **依赖注入** - 自动装配、生命周期管理、事件系统

## 快速开始

### 运行示例

查看完整功能演示：

```bash
# 运行 Web 应用示例 - 展示 Web 框架所有特性
cargo run -p web-demo

# 运行综合示例 - 展示依赖注入核心特性
cargo run -p app-demo

# 运行生命周期示例 - 展示 init/destroy 回调
cargo run --package chimera-core --example lifecycle_demo

# 测试环境变量覆盖
CHIMERA_PROFILES_ACTIVE=prod cargo run -p app-demo
```

### 最简示例

```rust
use chimera_core::prelude::*;
use chimera_core_macros::{bean, component, configuration, Component, Configuration};
use std::sync::Arc;

// 1. 定义一个服务组件
#[derive(Component)]
#[bean("userService")]
pub struct UserService {
    #[autowired]
    db: Arc<DatabaseService>,
}

#[component]
impl UserService {
    pub fn create_user(&self, name: &str) {
        println!("Creating user: {}", name);
    }
}

// 2. 定义配置类，创建第三方 Bean
#[derive(Configuration)]
pub struct AppConfig {
    #[autowired]
    environment: Arc<Environment>,
}

#[configuration]
impl AppConfig {
    #[bean]
    pub fn database_service(&self) -> DatabaseService {
        let url = self.environment
            .get_string("db.url")
            .unwrap_or_else(|| "localhost:5432".to_string());
        DatabaseService::connect(&url)
    }
}

// 3. 启动应用
#[tokio::main]
async fn main() -> ApplicationResult<()> {
    ChimeraApplication::new()
        .run()
        .await
}
```

框架会自动：
1. 扫描并注册所有 `@Component` 和 `@Configuration`
2. 解析依赖关系并按拓扑顺序初始化
3. 检测循环依赖并在启动时报错
4. 调用所有 `#[init]` 回调
5. 应用关闭时调用所有 `#[destroy]` 回调

### 添加依赖

在您的 `Cargo.toml` 中添加：

```toml
[dependencies]
# 核心依赖注入框架
chimera-core = "0.1"
chimera-core-macros = "0.1"

# Web 框架（可选）
chimera-web = "0.1"
chimera-web-macros = "0.1"

# 参数验证（Web 框架需要）
validator = { version = "0.18", features = ["derive"] }

# 模板引擎（可选，用于服务端渲染）
tera = "1"

# 异步运行时
tokio = { version = "1", features = ["full"] }

# 序列化（Web 应用需要）
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

### 基本使用流程

1. **定义配置** - 使用 `@ConfigurationProperties` 绑定配置，放在 `config/application.toml`
2. **定义服务** - 使用 `@Component` 标记组件，`@autowired` 注入依赖
3. **定义 Bean** - 使用 `@Configuration` + `@Bean` 创建第三方类型的 Bean
4. **定义 impl 块** - 使用 `@component` 或 `@configuration` 标记 impl 块（必须）
5. **启动应用** - 调用 `ChimeraApplication::new().run().await` 一行启动
6. **依赖自动处理** - 框架自动验证依赖、拓扑排序、初始化所有 Bean

详细代码示例请参考：
- `examples/app-demo` - 依赖注入、配置管理、事件系统示例
- `examples/web-demo` - Web 框架、Controller、参数验证、@Bean 方法示例

## 核心注解

### @component - Component impl 块标记

**所有使用 `#[derive(Component)]` 的类型，其 impl 块都必须添加 `#[component]` 属性**。

这个属性宏会自动检查方法名是否与 Component trait 的保留方法冲突，**在编译时直接报错**。

```rust
use chimera_core::prelude::*;
use chimera_core_macros::{component, Component};

#[derive(Component)]
#[bean("userService")]
struct UserService {
    #[autowired]
    db: Arc<DatabaseService>,
}

#[component]  // ✅ 必须添加
impl UserService {
    pub fn create_user(&self) { }   // ✅ OK
    pub fn user_register(&self) { } // ✅ OK
    pub fn register(&self) { }      // ❌ 编译错误：与 Component::register 冲突
}
```

**注意**：Controller 类型的 impl 块需要同时使用 `#[component]` 和 `#[controller]`：
- `#[component]` 负责方法名检查
- `#[controller]` 负责路由处理

```rust
#[controller("/api")]
#[derive(Component, Clone)]
pub struct ApiController { ... }

#[component]    // 方法名检查
#[controller]   // 路由处理
impl ApiController {
    #[get_mapping("/info")]
    async fn get_info(&self) -> impl IntoResponse { ... }
}
```

## 核心特性

### Web 框架 (Chimera Web)

基于 Axum 构建的 Spring Boot 风格 Web 框架：

- **@Controller** - 通过注解定义控制器，自动注册路由
- **自动路由** - 无需手动配置，方法注解自动生成路由
- **参数注入** - Spring Boot 风格的提取器：
  - `PathVariable<T>` - 从路径参数提取（类似 @PathVariable）
  - `RequestBody<T>` - 从 JSON body 反序列化（类似 @RequestBody）
  - `ValidatedRequestBody<T>` - 自动验证的 JSON body（类似 @Valid @RequestBody）
  - `RequestParam<T>` - 从 query 参数提取（类似 @RequestParam）
  - `ValidatedRequestParam<T>` - 自动验证的 query 参数（类似 @Valid @RequestParam）
  - `FormData<T>` - 从表单数据提取（类似 @ModelAttribute）
  - `ValidatedFormData<T>` - 自动验证的表单数据（类似 @Valid @ModelAttribute）
  - `Multipart` - 手动处理 multipart/form-data 文件上传
  - `MultipartForm<T>` - 自动提取文件和表单字段（类似 @ModelAttribute）
  - `ValidatedMultipartForm<T>` - 自动验证的提取文件和表单字段（类似 @Valid @ModelAttribute）
  - `RequestHeaders` - 提取 HTTP 请求头（类似 @RequestHeader）
- **文件上传** - 基于 multer 的文件上传支持，可配置文件大小限制
- **参数验证** - 基于标准 `validator` 库的自动验证，支持自定义错误消息
- **模板引擎** - 集成 Tera 模板引擎，支持热重载
- **全局异常处理** - 类似 Spring Boot 的 @ControllerAdvice
- **类型安全** - 编译时检查所有参数类型
- **依赖注入集成** - Controller 无缝访问 DI 容器中的 Bean

### 依赖注入 (Dependency Injection)

- **自动装配** - 通过 `@Component` 和 `@autowired` 注解实现类似 Spring 的自动依赖注入
- **@Configuration + @Bean** - 类似 Spring Boot 的配置类和 Bean 工厂方法
- **类型安全** - 基于 Rust 类型系统，编译时检查依赖关系
- **可选依赖** - 支持 `Option<Arc<T>>` 实现可选依赖注入
- **命名注入** - 支持通过 bean 名称进行精确注入
- **线程安全** - 使用 `Arc` 和 `RwLock` 保证并发安全
- **依赖验证** - 静态检测循环依赖和缺失依赖
- **拓扑排序** - 自动按依赖关系顺序初始化 Bean，无需手动排序

### 配置管理

- **@ConfigurationProperties** - 批量绑定配置到类型安全的结构体
- **@Value 注入** - 直接将配置值注入到字段
- **多配置源** - 支持 TOML 配置文件、环境变量等多种配置来源
- **自动查找配置** - 类似 Spring Boot，自动从 `config/application.toml` 加载
- **优先级管理** - 环境变量 > 配置文件 > 默认值
- **Profile 支持** - 类似 Spring 的 dev/prod 环境配置切换
- **配置命名空间** - 框架配置使用 `chimera.*` 前缀（如 `chimera.app.name`）

### Bean 作用域与生命周期

支持完整的 Bean 生命周期管理，包括作用域、延迟加载和回调：

- **Singleton** - 单例模式（默认），容器中只维护一个实例
- **Prototype** - 原型模式，每次获取创建新实例
- **Lazy** - 延迟初始化，首次使用时才创建 Bean
- **Init 回调** - Bean 初始化回调，类似 Spring 的 `@PostConstruct`
- **Destroy 回调** - Bean 销毁回调，类似 Spring 的 `@PreDestroy`
- **Shutdown Hooks** - 应用优雅关闭钩子

这些属性既可用于 `#[derive(Component)]` 组件，也可用于 `@Configuration` 中的 `@Bean` 方法：

```rust
use chimera_core::prelude::*;
use chimera_core_macros::{bean, configuration, destroy, init, lazy, scope, Configuration};

#[derive(Configuration)]
pub struct AppConfig {
    #[autowired]
    environment: Arc<Environment>,
}

#[configuration]
impl AppConfig {
    /// 默认：单例模式
    #[bean]
    pub fn email_service(&self) -> EmailService {
        EmailService::new()
    }

    /// 原型模式 - 每次获取创建新实例
    #[bean("counter")]
    #[scope("prototype")]
    pub fn counter_service(&self) -> u32 {
        0
    }

    /// 延迟初始化 - 首次使用时才创建
    #[bean("heavyService")]
    #[lazy]
    pub fn heavy_service(&self) -> HeavyService {
        HeavyService::new()
    }

    /// 带生命周期回调 - 使用默认方法名 init() 和 destroy()
    #[bean("dbPool")]
    #[init]
    #[destroy]
    pub fn database_pool(&self) -> DatabasePool {
        DatabasePool::new()
    }

    /// 自定义回调方法名
    #[bean("cache")]
    #[init("startup")]
    #[destroy("cleanup")]
    pub fn cache_manager(&self) -> CacheManager {
        CacheManager::new()
    }
}

impl DatabasePool {
    // #[init] 会自动调用此方法
    pub fn init(&mut self) -> ContainerResult<()> {
        tracing::info!("Initializing connection pool");
        Ok(())
    }

    // #[destroy] 会在应用关闭时调用
    pub fn destroy(&mut self) -> ContainerResult<()> {
        tracing::info!("Closing connection pool");
        Ok(())
    }
}

impl CacheManager {
    // #[init("startup")] 会调用此方法
    pub fn startup(&mut self) -> ContainerResult<()> {
        tracing::info!("Starting cache");
        Ok(())
    }

    // #[destroy("cleanup")] 会调用此方法
    pub fn cleanup(&mut self) -> ContainerResult<()> {
        tracing::info!("Cleaning up cache");
        Ok(())
    }
}
```

### 模板引擎

基于 Tera 的服务端模板渲染引擎（类似 Jinja2/Django Templates）：

- **Tera 模板引擎** - 功能强大的模板语法，支持变量、循环、条件、过滤器等
- **热重载** - 开发模式下自动监听模板文件变化，无需重启服务器
- **依赖注入集成** - TemplateEngine 作为 Bean 管理，通过 `#[autowired]` 自动注入
- **类型安全** - 通过 `Template` 构建器提供类型安全的模板渲染
- **配置化** - 通过配置文件控制模板目录、热重载等选项
- **优雅 API** - 链式调用，自动实现 `IntoResponse`

```rust
use chimera_web::prelude::*;
use std::sync::Arc;

#[controller("/templates")]
#[derive(Component, Clone)]
pub struct TemplateController {
    // 注入 TemplateEngine
    #[autowired]
    template_engine: Arc<TemplateEngine>,
}

#[component]
#[controller]
impl TemplateController {
    // 渲染模板并传递数据
    #[get_mapping("/home")]
    async fn home(&self) -> impl IntoResponse {
        self.template_engine.render("index.html")
            .with("title", "Chimera Web Framework")
            .with("message", "Welcome to Chimera!")
    }

    // 渲染列表数据
    #[get_mapping("/users")]
    async fn users(&self) -> impl IntoResponse {
        let users = vec![
            User { id: 1, name: "Alice".to_string() },
            User { id: 2, name: "Bob".to_string() },
        ];
        self.template_engine.render("users.html")
            .with("users", &users)
    }

    // 自定义状态码
    #[get_mapping("/error")]
    async fn error(&self) -> impl IntoResponse {
        self.template_engine.render("error.html")
            .with("error_code", 404)
            .with("message", "Page not found")
            .status(StatusCode::NOT_FOUND)
    }
}
```

**配置示例**（`config/application.toml`）：

```toml
[chimera.tera]
# 是否启用 Tera 模板引擎（启用后会自动注册 TemplateEngine Bean）
enabled = true

# 模板目录（相对于应用运行目录）
template-dir = "templates"

# 模板文件匹配模式
pattern = "templates/**/*"

# 是否启用热重载（开发模式建议开启，生产环境建议关闭）
# 启用后会自动监听模板文件变化，修改后无需重启服务器
hot-reload = true
```

**特性说明**：

- **自动注册 Bean**：启用后，`TemplateEngine` 会自动注册到依赖注入容器，可在任何 Controller 中通过 `#[autowired]` 注入
- **热重载**：开发模式下修改模板文件后立即生效，无需重启服务器，提升开发效率
- **线程安全**：使用 `Arc<RwLock<Tera>>` 实现，支持多线程并发访问
- **错误处理**：模板渲染错误会自动转换为 HTTP 500 响应

**模板示例**（`templates/users.html`）：

```html
<!DOCTYPE html>
<html>
<head>
    <title>用户列表</title>
</head>
<body>
    <h1>用户列表</h1>
    <ul>
    {% for user in users %}
        <li>{{ user.name }} (ID: {{ user.id }})</li>
    {% endfor %}
    </ul>
</body>
</html>
```

### @Configuration + @Bean 工厂方法

类似 Spring Boot 的 `@Configuration` + `@Bean`，用于手动定义和配置 Bean。

#### 基本用法

```rust
use chimera_core::prelude::*;
use chimera_core_macros::{bean, configuration, Configuration};

// 1. 使用 #[derive(Configuration)] 标记配置类
#[derive(Configuration)]
pub struct AppConfig {
    // 配置类本身也是 Component，支持依赖注入
    #[autowired]
    environment: Arc<Environment>,
    #[autowired]
    context: Arc<ApplicationContext>,
}

// 2. 在 impl 块上使用 #[configuration] 属性
#[configuration]
impl AppConfig {
    /// 定义 Bean 工厂方法
    ///
    /// - 方法名作为 bean 名称（email_service）
    /// - 返回值类型作为 bean 类型
    /// - 可以返回 T 或 ContainerResult<T>
    #[bean]
    pub fn email_service(&self) -> EmailService {
        let host = self.environment
            .get_string("email.smtp.host")
            .unwrap_or_else(|| "localhost".to_string());
        EmailService::new(host)
    }

    /// 自定义 Bean 名称
    #[bean("customSms")]
    pub fn sms_service(&self) -> SmsService {
        SmsService::new()
    }

    /// 返回 Result 类型，框架会自动处理错误
    #[bean]
    pub fn database_service(&self) -> ContainerResult<DatabaseService> {
        let url = self.environment
            .get_string("db.url")
            .ok_or_else(|| ContainerError::Other(anyhow::anyhow!("db.url not found")))?;
        DatabaseService::connect(&url)
    }

    /// Bean 之间的依赖注入
    /// 通过 ApplicationContext 获取其他 Bean
    #[bean]
    pub fn notification_service(&self) -> ContainerResult<NotificationService> {
        Ok(NotificationService::new(
            self.context.get_bean_by_type::<EmailService>()?,
            self.context.get_bean_by_type::<SmsService>()?,
        ))
    }
}
```

#### 支持的属性

Bean 方法支持以下属性（类似 Component）：

```rust
use chimera_core_macros::{bean, configuration, destroy, init, lazy, scope, Configuration};

#[configuration]
impl AppConfig {
    /// 原型作用域 - 每次获取创建新实例
    #[bean("counter")]
    #[scope("prototype")]
    pub fn counter(&self) -> u32 { 0 }

    /// 延迟初始化 - 首次使用时才创建
    #[bean("heavyService")]
    #[lazy]
    pub fn heavy_service(&self) -> HeavyService {
        HeavyService::new()
    }

    /// 初始化和销毁回调 - 使用默认方法名
    #[bean("dbPool")]
    #[init]           // 调用 bean.init()
    #[destroy]        // 调用 bean.destroy()
    pub fn db_pool(&self) -> ConnectionPool {
        ConnectionPool::new()
    }

    /// 自定义回调方法名
    #[bean("cache")]
    #[init("startup")]      // 调用 bean.startup()
    #[destroy("cleanup")]   // 调用 bean.cleanup()
    pub fn cache(&self) -> CacheManager {
        CacheManager::new()
    }

    /// 组合使用多个属性
    #[bean("expensiveService")]
    #[scope("singleton")]
    #[lazy]
    #[init("initialize")]
    #[destroy("shutdown")]
    pub fn expensive_service(&self) -> ExpensiveService {
        ExpensiveService::new()
    }
}

// 生命周期方法实现
impl ConnectionPool {
    pub fn init(&mut self) -> ContainerResult<()> {
        tracing::info!("Initializing connection pool");
        Ok(())
    }

    pub fn destroy(&mut self) -> ContainerResult<()> {
        tracing::info!("Closing connection pool");
        Ok(())
    }
}

impl CacheManager {
    pub fn startup(&mut self) -> ContainerResult<()> {
        tracing::info!("Starting cache");
        Ok(())
    }

    pub fn cleanup(&mut self) -> ContainerResult<()> {
        tracing::info!("Cleaning up cache");
        Ok(())
    }
}
```

#### 与 @Component 的区别

| 特性 | @Component | @Configuration + @Bean |
|------|-----------|----------------------|
| 定义方式 | 派生宏，自动扫描 | 手动工厂方法 |
| 依赖注入 | 字段注入 `#[autowired]` | 方法参数或 ApplicationContext |
| 适用场景 | 自己开发的类 | 第三方类型、需要复杂初始化逻辑 |
| 初始化控制 | 自动构造 | 完全手动控制 |
| Bean 名称 | `#[bean("name")]` 或类型名 | 方法名或 `#[bean("name")]` |
| 作用域 | `#[derive(Component)] #[scope("...")]` | `#[bean] #[scope("...")]` |
| 生命周期 | `#[init]` `#[destroy]` | `#[init]` `#[destroy]` |

### 事件系统

- **同步/异步事件** - 支持同步和异步两种事件处理模式
- **ApplicationEventPublisher** - 事件发布接口
- **ApplicationEventMulticaster** - 事件分发机制
- **EventListener** - 通用事件监听器
- **TypedEventListener** - 类型化事件监听器
- **内置应用事件** - ApplicationStartedEvent、ApplicationShutdownEvent 等
- **异常处理** - 支持 ErrorHandler 统一处理监听器异常

### 核心组件注入

框架自动注册以下核心组件，可通过 `@autowired` 直接注入使用：

- **ApplicationContext** - 应用上下文，动态获取 Bean
- **Environment** - 配置环境，访问配置源、激活的 Profile
- **ApplicationEventPublisher** - 事件发布器，发布自定义事件

### 应用启动器

- **ChimeraApplication** - Spring Boot 风格的一行启动方式
- **智能阻塞** - 有 keep-alive 插件（如 Web 服务器）时自动阻塞，否则执行完退出
- **自动组件扫描** - 自动发现并注册所有标记 `@Component` 和 `@Configuration` 的组件
- **配置自动加载** - 自动加载配置文件和环境变量
- **依赖自动验证** - 启动时自动验证所有依赖关系，检测循环依赖
- **拓扑排序初始化** - 基于依赖关系自动确定 Bean 初始化顺序，被依赖的 Bean 先初始化
- **Banner 显示** - 启动时显示框架信息
- **插件机制** - 支持自定义插件扩展框架功能

### 日志系统

- **基于 tracing** - 使用 Rust 生态标准的 tracing 框架
- **自动初始化** - 应用启动时自动配置日志
- **多级别支持** - 支持 TRACE、DEBUG、INFO、WARN、ERROR 日志级别
- **灵活配置** - 通过环境变量 `RUST_LOG` 控制日志级别

## 项目结构

```
chimera/
├── chimera-core/          # 核心依赖注入框架
├── chimera-core-macros/   # 核心宏定义
├── chimera-web/           # Web 框架
├── chimera-web-macros/    # Web 宏定义
└── examples/
    ├── app-demo/          # 依赖注入示例
    │   ├── src/
    │   └── config/
    │       └── application.toml
    └── web-demo/          # Web 框架示例
        ├── src/
        └── config/
            └── application.toml
```

### 配置文件目录结构

类似 Spring Boot，推荐使用 `config/` 目录存放配置文件：

```
your-project/
├── Cargo.toml
├── config/                 # 配置目录（与 src 同级）
│   ├── application.toml    # 默认配置
│   ├── application-dev.toml   # 开发环境
│   └── application-prod.toml  # 生产环境
└── src/
    └── main.rs
```

**自动查找规则**（无需手动指定）：
1. 优先查找 `config/application.toml`
2. 如果不存在，查找 `application.toml`
3. 支持 Profile：`config/application-{profile}.toml`

## 核心注解说明

### 依赖注入注解

| 注解 | 作用 | 示例 |
|------|------|------|
| `#[derive(Component)]` | 标记为自动装配组件 | 服务类、仓库类 |
| `#[component]` | 标记 Component 的 impl 块 | 必须用于 impl 块 |
| `#[derive(Configuration)]` | 标记为配置类（特殊的 Component） | 包含 Bean 工厂方法的配置类 |
| `#[configuration]` | 标记 Configuration 的 impl 块 | 自动扫描 @Bean 方法 |
| `#[bean]` 或 `#[bean("name")]` | 标记 Bean 工厂方法 | Configuration 中定义 Bean |
| `#[derive(ConfigurationProperties)]` | 批量绑定配置 | 配置类 |
| `#[autowired]` | 自动注入依赖 | 字段依赖注入 |
| `#[autowired("beanName")]` | 按名称注入依赖 | 命名 Bean 注入 |
| `#[value("config.key")]` | 注入配置值 | 单个配置注入 |
| `#[scope("singleton")]` | 指定作用域 | singleton/prototype |
| `#[lazy]` | 延迟初始化 | 按需加载 Bean |
| `#[init]` 或 `#[init("method")]` | 初始化回调 | Bean 创建后执行 |
| `#[destroy]` 或 `#[destroy("method")]` | 销毁回调 | Bean 销毁前执行 |
| `#[event_listener]` | 事件监听器 | 监听应用事件 |

**⚠️ Component 保留方法名**

使用 `#[derive(Component)]` 的类型会自动实现 `Component` trait，该 trait 保留了以下方法名：

- `bean_name()`, `scope()`, `lazy()`, `dependencies()`
- `init_callback()`, `destroy_callback()`
- `is_event_listener()`, `as_event_listener()`
- `create_from_context()`, `register()`

**在 Component impl 块中使用这些方法名会导致编译错误**，例如：

```rust
// ❌ 错误：register 与 Component trait 冲突
#[post_mapping("/register")]
async fn register(&self, ...) { }

// ✅ 正确：使用不同的方法名
#[post_mapping("/register")]
async fn user_register(&self, ...) { }
```

框架使用 `#[component]` 属性宏在编译时检查方法名，如果使用了保留方法名会立即报错。

### Web 框架注解

| 注解 | 作用 | 示例 |
|------|------|------|
| `#[controller("/path")]` | 标记结构体为控制器并指定基础路径 | `#[controller("/api")]` |
| `#[controller]` | 标记 impl 块为控制器实现 | 自动注册方法路由 |
| `#[get_mapping("/path")]` | 映射 GET 请求 | 查询操作 |
| `#[post_mapping("/path")]` | 映射 POST 请求 | 创建操作 |
| `#[put_mapping("/path")]` | 映射 PUT 请求 | 更新操作 |
| `#[delete_mapping("/path")]` | 映射 DELETE 请求 | 删除操作 |
| `#[patch_mapping("/path")]` | 映射 PATCH 请求 | 部分更新 |

### 验证规则

基于 Rust 标准 `validator` 库，支持以下验证规则：

| 验证规则 | 说明 |
|---------|------|
| `length(min = X, max = Y, message = "...")` | 字符串长度 |
| `email(message = "...")` | 邮箱格式 |
| `range(min = X, max = Y, message = "...")` | 数值范围 |
| `regex(path = "*REGEX", message = "...")` | 正则匹配 |
| `url(message = "...")` | URL 格式 |
| `must_match(other = "field", message = "...")` | 字段匹配 |
| `contains(pattern = "...", message = "...")` | 包含子串 |

更多验证规则请参考 [validator 文档](https://docs.rs/validator/)

## 框架配置

框架配置使用 `chimera.*` 命名空间：

```toml
[chimera.app]
name = "MyApp"
version = "1.0.0"

[chimera.events]
async = false  # 是否异步处理事件

[chimera.profiles]
active = ["dev"]  # 激活的 profiles

[chimera.web.multipart]
max-file-size = 10485760  # 最大文件大小（字节），默认 10MB
max-fields = 100          # 最大字段数量，默认 100

[chimera.tera]
enabled = true            # 是否启用 Tera 模板引擎
template-dir = "templates"  # 模板目录
pattern = "templates/**/*"  # 模板文件匹配模式
hot-reload = true         # 是否启用热重载（开发模式建议开启）
```

环境变量前缀为 `CHIMERA_`，例如：
- `CHIMERA_PROFILES_ACTIVE=prod` - 设置激活的 profile

## 示例场景

框架适用于以下场景：

- **RESTful API** - 使用 Chimera Web 快速构建类型安全的 REST API
- **Web 应用** - 完整的 Web 应用开发，包括表单处理、文件上传等
- **微服务** - 构建可配置、可测试的微服务应用
- **后台任务** - 定时任务、消息队列消费者等
- **命令行工具** - 复杂的企业级 CLI 工具
- **数据处理** - 批量数据处理、ETL 任务等

## 设计原则

- **类型安全** - 充分利用 Rust 类型系统，编译期检查
- **零成本抽象** - 尽可能在编译期完成，运行时开销最小
- **线程安全** - 所有 Bean 都是 `Send + Sync`
- **惯用 Rust** - 遵循 Rust 最佳实践和编码规范
- **渐进式** - 支持从简单到复杂的渐进式使用

## 后续规划

### Web 框架
- [ ] 添加 WebSocket 支持
- [ ] 支持 OpenAPI/Swagger 文档自动生成
- [ ] 支持 gRPC

## 贡献

欢迎提交 Issue 和 Pull Request！

## 许可

MIT OR Apache-2.0

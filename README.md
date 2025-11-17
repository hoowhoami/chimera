# Chimera

一个受 Spring Boot 启发的 Rust 应用开发框架，提供依赖注入、Web 服务器、配置管理等企业级功能，让您以类型安全、线程安全的方式快速构建 Rust 应用。

## 特点

- **Spring Boot 风格** - 熟悉的注解和开发体验
- **类型安全** - 编译期依赖检查，运行时零成本
- **线程安全** - 所有 Bean 都是 `Send + Sync`
- **Web 框架** - 基于 Axum 的高性能 Web 服务器
- **配置管理** - 多源配置、环境切换、类型绑定
- **依赖注入** - 自动装配、生命周期管理、事件系统

## 核心特性

### Web 框架 (Chimera Web)

基于 Axum 构建的 Spring Boot 风格 Web 框架：

- **@Controller** - 通过注解定义控制器，自动注册路由
- **自动路由** - 无需手动配置，方法注解自动生成路由
- **参数注入** - Spring Boot 风格的提取器：
  - `Autowired<T>` - 从 DI 容器注入 Bean（类似 @Autowired）
  - `PathVariable<T>` - 从路径参数提取（类似 @PathVariable）
  - `RequestBody<T>` - 从 JSON body 反序列化（类似 @RequestBody）
  - `ValidatedRequestBody<T>` - 自动验证的 JSON body（类似 @Valid @RequestBody）
  - `RequestParam<T>` - 从 query 参数提取（类似 @RequestParam）
  - `ValidatedRequestParam<T>` - 自动验证的 query 参数（类似 @Valid @RequestParam）
  - `FormData<T>` - 从表单数据提取（类似 @ModelAttribute）
  - `ValidatedFormData<T>` - 自动验证的表单数据（类似 @Valid @ModelAttribute）
  - `RequestHeaders` - 提取 HTTP 请求头（类似 @RequestHeader）
- **参数验证** - 基于 `chimera_validator::Validate` 的自动验证
- **分层错误处理** - 提取器、中间件、业务逻辑的分层错误处理
- **全局异常处理** - 类似 Spring Boot 的 @ControllerAdvice
- **类型安全** - 编译时检查所有参数类型
- **依赖注入集成** - Controller 无缝访问 DI 容器中的 Bean
- **灵活组合** - 在一个方法中使用多个提取器

### 依赖注入 (Dependency Injection)

- **自动装配** - 通过 `@Component` 和 `@autowired` 注解实现类似 Spring 的自动依赖注入
- **类型安全** - 基于 Rust 类型系统，编译时检查依赖关系
- **可选依赖** - 支持 `Option<Arc<T>>` 实现可选依赖注入，服务降级更灵活
- **命名注入** - 支持通过 bean 名称进行精确注入
- **线程安全** - 使用 `Arc` 和 `RwLock` 保证并发安全
- **依赖验证** - 静态检测循环依赖和缺失依赖，提前发现问题

### 配置管理

- **@ConfigurationProperties** - 批量绑定配置到类型安全的结构体，自动注册为 Bean
- **@Value 注入** - 直接将配置值注入到字段，支持默认值和多种数据类型（String、i32、Vec等）
- **多配置源** - 支持 TOML 配置文件、环境变量等多种配置来源
- **自动查找配置** - 类似 Spring Boot，自动从 `config/application.toml` 或 `application.toml` 加载
- **优先级管理** - 环境变量 > 配置文件 > 默认值，灵活覆盖配置
- **Profile 支持** - 类似 Spring 的 dev/prod 环境配置切换（`config/application-dev.toml`）
- **字段名转换** - 自动将 snake_case 转换为 kebab-case
- **数组支持** - @Value 支持 Vec 类型，兼容 TOML 数组和逗号分隔字符串

### Bean 作用域与生命周期

- **Singleton** - 单例模式，容器中只维护一个实例
- **Prototype** - 原型模式，每次获取创建新实例
- **Lazy** - 延迟初始化，按需创建 Bean
- **@init** - Bean 初始化回调，类似 Spring 的 `@PostConstruct`
- **@destroy** - Bean 销毁回调，类似 Spring 的 `@PreDestroy`
- **Shutdown Hooks** - 应用优雅关闭钩子

### 事件系统

- **异步事件发布/订阅** - 基于 tokio 的异步事件处理机制
- **EventListener** - 通用事件监听器，监听所有事件
- **TypedEventListener** - 类型化事件监听器，只监听特定类型事件
- **内置应用事件** - ApplicationStartedEvent、ApplicationShutdownEvent 等
- **自定义事件** - 轻松定义和发布业务事件

### 核心组件注入

框架自动注册以下核心组件，可通过 `@autowired` 直接注入使用：

- **ApplicationContext** - 应用上下文，动态获取 Bean、检查 Bean 存在性
- **Environment** - 配置环境，访问配置源、激活的 Profile
- **AsyncEventPublisher** - 事件发布器，发布自定义事件

### 应用启动器

- **ChimeraApplication** - Spring Boot 风格的一行启动方式
- **自动组件扫描** - 自动发现并注册所有标记 `@Component` 的组件
- **配置自动加载** - 自动加载配置文件和环境变量
- **依赖自动验证** - 启动时自动验证所有依赖关系
- **Banner 显示** - 启动时显示框架信息
- **初始化器** - 支持自定义初始化逻辑

### 日志系统

- **基于 tracing** - 使用 Rust 生态标准的 tracing 框架
- **自动初始化** - 应用启动时自动配置日志
- **多级别支持** - 支持 TRACE、DEBUG、INFO、WARN、ERROR 日志级别
- **灵活配置** - 通过环境变量 `RUST_LOG` 控制日志级别和过滤器

## 快速开始

### 运行示例

查看完整功能演示：

```bash
# 运行 Web 应用示例 - 展示 Web 框架所有特性
cargo run -p web-demo

# 运行综合示例 - 展示依赖注入核心特性
cargo run -p app-demo

# 测试环境变量覆盖
DEMO_DATABASE_URL=custom cargo run -p app-demo
DEMO_SERVER_PORT=9000 cargo run -p app-demo
```

### Web 应用开发

使用 Chimera Web 构建 RESTful API：

```rust
use chimera_core::prelude::*;
use chimera_core_macros::Component;
use chimera_web::prelude::*;
use chimera_web_macros::{Controller, controller, get_mapping, post_mapping};
use chimera_web::extractors::{PathVariable, RequestBody};

// 1. 定义 Controller
#[derive(Controller, Component, Clone)]
#[route("/api/users")]
struct UserController {
    #[autowired]
    user_service: Arc<UserService>,
}

#[controller]
impl UserController {
    // GET /api/users/:id - 使用 PathVariable 提取路径参数
    #[get_mapping("/:id")]
    async fn get_user(&self, PathVariable(id): PathVariable<u32>) -> impl IntoResponse {
        match self.user_service.find_by_id(id).await {
            Some(user) => ResponseEntity::ok(user),
            None => ResponseEntity::not_found(json!({"error": "User not found"}))
        }
    }

    // POST /api/users - 使用 RequestBody 提取 JSON
    #[post_mapping("/")]
    async fn create_user(&self, RequestBody(req): RequestBody<CreateUserRequest>) -> impl IntoResponse {
        let user = self.user_service.create(req).await;
        ResponseEntity::created(user)
    }
}

// 2. 启动应用（一行启动，自动阻塞）
#[tokio::main]
async fn main() -> ApplicationResult<()> {
    ChimeraApplication::new("MyApp")
        .run_until_shutdown()  // 类似 Spring Boot 的 SpringApplication.run()
        .await
}
```

### 参数验证

使用 `ValidatedRequestBody` 自动验证请求参数（类似 Spring Boot 的 `@Valid @RequestBody`）：

```rust
use chimera_validator::{Validate, Length, Email, Range};
use chimera_web::extractors::ValidatedRequestBody;

// 1. 定义带验证规则的请求模型
#[derive(Deserialize, Validate)]
struct CreateUserRequest {
    #[validate(length(min = 2, max = 20, message = "用户名长度必须在2-20个字符之间"))]
    username: String,

    #[validate(email(message = "邮箱格式不正确"))]
    email: String,

    #[validate(range(min = 18, max = 120, message = "年龄必须在18-120之间"))]
    age: u8,
}

// 2. 使用 ValidatedRequestBody 自动验证
#[controller]
impl UserController {
    #[post_mapping("/register")]
    async fn register(&self, ValidatedRequestBody(req): ValidatedRequestBody<CreateUserRequest>) -> impl IntoResponse {
        // 如果执行到这里，说明验证已通过
        // 验证失败会自动返回 400 Bad Request 和详细的验证错误信息
        let user = self.user_service.create(req).await;
        ResponseEntity::created(user)
    }
}
```

**验证失败时的响应示例**：
```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "status": 400,
  "error": "ValidationError",
  "message": "Validation failed",
  "path": "/api/users/register",
  "details": {
    "field_errors": {
      "username": ["用户名长度必须在2-20个字符之间"],
      "email": ["邮箱格式不正确"],
      "age": ["年龄必须在18-120之间"]
    }
  }
}
```

### 全局异常处理

类似 Spring Boot 的 `@ControllerAdvice`，实现自定义异常处理器：

```rust
use chimera_web::exception_handler::{GlobalExceptionHandler, WebError, ErrorResponse};
use chimera_web_macros::ExceptionHandler;

// 1. 定义业务错误类型
#[derive(Error, Debug)]
pub enum BusinessError {
    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("Database error: {0}")]
    DatabaseError(String),
}

// 2. 实现全局异常处理器
#[derive(ExceptionHandler, Component)]
#[bean("businessExceptionHandler")]
pub struct BusinessExceptionHandler {
    #[value("app.debug", default = false)]
    debug_mode: bool,
}

#[async_trait]
impl GlobalExceptionHandler for BusinessExceptionHandler {
    fn name(&self) -> &str {
        "BusinessExceptionHandler"
    }

    fn priority(&self) -> i32 {
        10 // 高优先级
    }

    fn can_handle(&self, error: &WebError) -> bool {
        matches!(error, WebError::UserDefined(_))
    }

    async fn handle_error(&self, error: &WebError, request_path: &str) -> Option<ErrorResponse> {
        match error {
            WebError::UserDefined(e) => {
                if let Some(business_error) = e.downcast_ref::<BusinessError>() {
                    let (status_code, error_type) = match business_error {
                        BusinessError::UserNotFound(_) => {
                            (StatusCode::NOT_FOUND, "UserNotFound")
                        }
                        BusinessError::DatabaseError(_) => {
                            (StatusCode::INTERNAL_SERVER_ERROR, "DatabaseError")
                        }
                    };

                    Some(ErrorResponse::new(
                        status_code,
                        error_type.to_string(),
                        business_error.to_string(),
                        request_path.to_string(),
                    ))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

// 3. 在 Controller 中使用业务错误
#[controller]
impl UserController {
    #[get_mapping("/:id")]
    async fn get_user(&self, PathVariable(id): PathVariable<u32>) -> impl IntoResponse {
        match self.user_service.find_by_id(id).await {
            Some(user) => ResponseEntity::ok(user),
            None => {
                let error = BusinessError::UserNotFound(id.to_string());
                WebError::UserDefined(Box::new(error)).into_response()
            }
        }
    }
}
```

**错误处理层级**（类似 Axum 的分层架构）：

1. **提取器层级** - 请求参数解析错误（JSON、Path、Query、Form 等）
2. **中间件层级** - 认证、授权、限流等错误
3. **业务逻辑层级** - Handler 函数中的业务错误
4. **全局处理层级** - 统一捕获和转换所有错误
5. **框架底层层级** - HTTP 服务器、连接错误

```

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

# 异步运行时
tokio = { version = "1", features = ["full"] }

# 序列化（Web 应用需要）
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

### 基本使用流程

1. **定义配置** - 使用 `@ConfigurationProperties` 绑定配置，放在 `config/application.toml`
2. **定义服务** - 使用 `@Component` 标记组件，`@autowired` 注入依赖
3. **启动应用** - 调用 `ChimeraApplication::new().run_until_shutdown()` 一行启动
4. **使用服务** - 框架自动注册路由，或从 ApplicationContext 获取 Bean 并调用

## 项目结构

```
chimera/
├── chimera-core/          # 核心依赖注入框架
│   ├── application.rs     # 应用启动器
│   ├── context.rs         # ApplicationContext 容器
│   ├── config.rs          # 配置管理
│   └── events.rs          # 事件系统
├── chimera-core-macros/   # 核心宏定义
│   ├── component.rs       # @Component 宏
│   └── config.rs          # @ConfigurationProperties 宏
├── chimera-web/           # Web 框架
│   ├── server.rs          # Web 服务器
│   ├── extractors.rs      # 参数提取器
│   ├── controller.rs      # Controller 特质
│   └── middleware.rs      # 中间件
├── chimera-web-macros/    # Web 宏定义
│   ├── controller.rs      # @Controller 宏
│   └── route.rs           # 路由映射宏
└── examples/
    ├── app-demo/          # 依赖注入示例
    │   ├── src/
    │   └── config/        # 配置目录（推荐）
    │       ├── application.toml
    │       └── application-dev.toml
    └── web-demo/          # Web 框架示例
        ├── src/
        └── config/        # 配置目录（推荐）
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
| `#[derive(ConfigurationProperties)]` | 批量绑定配置 | 配置类 |
| `#[autowired]` | 自动注入依赖 | 字段依赖注入 |
| `#[autowired("beanName")]` | 按名称注入依赖 | 命名 Bean 注入 |
| `#[value("config.key")]` | 注入配置值 | 单个配置注入 |
| `#[bean("name")]` | 指定 Bean 名称 | 自定义 Bean 标识 |
| `#[scope("singleton")]` | 指定作用域 | singleton/prototype |
| `#[lazy]` | 延迟初始化 | 按需加载 Bean |
| `#[init]` | 初始化回调 | Bean 创建后执行 |
| `#[destroy]` | 销毁回调 | Bean 销毁前执行 |
| `#[event_listener]` | 事件监听器 | 监听应用事件 |

### Web 框架注解

| 注解 | 作用 | 示例 |
|------|------|------|
| `#[derive(Controller)]` | 标记为控制器 | 定义 REST API |
| `#[route("/path")]` | 指定控制器基础路径 | `/api`, `/users` |
| `#[controller]` | 标记 impl 块为控制器实现 | 自动注册方法路由 |
| `#[get_mapping("/path")]` | 映射 GET 请求 | 查询操作 |
| `#[post_mapping("/path")]` | 映射 POST 请求 | 创建操作 |
| `#[put_mapping("/path")]` | 映射 PUT 请求 | 更新操作 |
| `#[delete_mapping("/path")]` | 映射 DELETE 请求 | 删除操作 |
| `#[patch_mapping("/path")]` | 映射 PATCH 请求 | 部分更新 |
| `#[request_mapping("/path")]` | 映射所有 HTTP 方法 | 通用处理 |

### Web 提取器

在 controller 方法中直接使用，用于提取请求参数：

| 提取器 | 作用 | Spring Boot 等价 |
|--------|------|------------------|
| `PathVariable<T>` | 提取路径参数 | `@PathVariable` |
| `RequestBody<T>` | 提取 JSON body | `@RequestBody` |
| `RequestParam<T>` | 提取 query 参数 | `@RequestParam` |
| `FormData<T>` | 提取表单数据 | `@ModelAttribute` |
| `RequestHeaders` | 提取请求头 | `@RequestHeader` |
| `Autowired<T>` | 注入 Bean | `@Autowired` |

## 示例场景

框架适用于以下场景：

- **RESTful API** - 使用 Chimera Web 快速构建类型安全的 REST API
- **Web 应用** - 完整的 Web 应用开发，包括表单处理、文件上传等
- **微服务** - 构建可配置、可测试的微服务应用
- **后台任务** - 定时任务、消息队列消费者等
- **命令行工具** - 复杂的企业级 CLI 工具
- **数据处理** - 批量数据处理、ETL 任务等

## 核心概念

### 容器 (Container)

ApplicationContext 是核心容器，负责：
- Bean 的创建、缓存和生命周期管理
- 依赖关系解析和注入
- 配置管理和环境变量处理
- 事件发布和监听器管理

### Bean

Bean 是容器管理的对象实例，特点：
- 由容器创建和管理生命周期
- 支持依赖注入
- 可配置作用域（单例/原型）
- 支持生命周期回调

### 依赖解析

框架在启动时：
1. 扫描所有 `@Component` 标记的组件
2. 分析每个组件的 `@autowired` 依赖
3. 验证依赖关系（检测循环依赖、缺失依赖）
4. 按依赖顺序创建 Bean
5. 自动注入依赖到字段

### Web 架构

Chimera Web 基于 Axum 构建，在启动时：
1. 扫描所有标记 `@Controller` 的控制器
2. 从 DI 容器中获取控制器实例
3. 解析每个控制器方法的路由映射注解
4. 自动生成路由处理函数，支持参数提取器
5. 注册到 Axum Router
6. 启动 HTTP 服务器

**路由注册流程**：
```
@Controller -> 扫描方法 -> 解析注解 -> 生成 handler -> 注入提取器 -> 注册到 Router
```

**请求处理流程**：
```
HTTP Request -> Router 匹配 -> 提取器解析参数 -> 调用 controller 方法 -> 返回 Response
```

## 设计原则

- **类型安全** - 充分利用 Rust 类型系统，编译期检查
- **零成本抽象** - 尽可能在编译期完成，运行时开销最小
- **线程安全** - 所有 Bean 都是 `Send + Sync`
- **惯用 Rust** - 遵循 Rust 最佳实践和编码规范
- **渐进式** - 支持从简单到复杂的渐进式使用


## 后续规划

### 核心框架
- [ ] 添加容器启动性能分析工具
- [ ] 支持 AOP 切面编程
- [ ] 提供 Bean 工厂扩展机制

### Web 框架
- [ ] 添加文件上传支持（multipart/form-data）
- [ ] 实现 Cookie 和 Session 提取器
- [ ] 添加 WebSocket 支持
- [x] 实现全局异常处理器
- [x] 实现类似 Spring Validate 的参数验证
- [ ] 支持 OpenAPI/Swagger 文档自动生成
- [ ] 添加速率限制中间件
- [ ] 支持 gRPC

## 贡献

欢迎提交 Issue 和 Pull Request！

## 许可

MIT OR Apache-2.0

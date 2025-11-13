# Chimera

一个受 Spring Boot 启发的 Rust 依赖注入框架，提供类型安全、线程安全的企业级应用开发体验。

## 核心特性

### 依赖注入 (Dependency Injection)

- **自动装配** - 通过 `@Component` 和 `@autowired` 注解实现类似 Spring 的自动依赖注入
- **类型安全** - 基于 Rust 类型系统，编译时检查依赖关系
- **可选依赖** - 支持 `Option<Arc<T>>` 实现可选依赖注入，服务降级更灵活
- **命名注入** - 支持通过 bean 名称进行精确注入
- **线程安全** - 使用 `Arc` 和 `RwLock` 保证并发安全
- **依赖验证** - 静态检测循环依赖和缺失依赖，提前发现问题

### 配置管理

- **@ConfigurationProperties** - 批量绑定配置到类型安全的结构体，自动注册为 Bean
- **@Value 注入** - 直接将配置值注入到字段，支持默认值
- **多配置源** - 支持 TOML 配置文件、环境变量等多种配置来源
- **优先级管理** - 环境变量 > 配置文件 > 默认值，灵活覆盖配置
- **Profile 支持** - 类似 Spring 的 dev/prod 环境配置切换
- **字段名转换** - 自动将 snake_case 转换为 kebab-case

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
# 运行综合示例 - 展示所有核心特性
cargo run -p app-demo

# 测试环境变量覆盖
DEMO_DATABASE_URL=custom cargo run -p app-demo
DEMO_SERVER_PORT=9000 cargo run -p app-demo
```

### 添加依赖

```toml
[dependencies]
chimera-core = { path = "chimera-core" }
chimera-macros = { path = "chimera-macros" }
inventory = "0.3"  # 自动组件扫描需要
```

### 基本使用流程

1. **定义配置** - 使用 `@ConfigurationProperties` 绑定配置
2. **定义服务** - 使用 `@Component` 标记组件，`@autowired` 注入依赖
3. **启动应用** - 调用 `ChimeraApplication::new().run()` 一行启动
4. **使用服务** - 从 ApplicationContext 获取 Bean 并调用

## 核心注解说明

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

## 示例场景

框架适用于以下场景：

- **Web 应用** - 结合 Actix-web、Axum 等 Web 框架构建 RESTful API
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

## 设计原则

- **类型安全** - 充分利用 Rust 类型系统，编译期检查
- **零成本抽象** - 尽可能在编译期完成，运行时开销最小
- **线程安全** - 所有 Bean 都是 `Send + Sync`
- **惯用 Rust** - 遵循 Rust 最佳实践和编码规范
- **渐进式** - 支持从简单到复杂的渐进式使用

## 与 Spring Boot 对比

| 功能 | Spring Boot | Chimera |
|------|-------------|---------|
| 依赖注入 | @Autowired | #[autowired] |
| 组件扫描 | @Component | #[derive(Component)] |
| 配置绑定 | @ConfigurationProperties | #[derive(ConfigurationProperties)] |
| 配置注入 | @Value | #[value] |
| 生命周期 | @PostConstruct/@PreDestroy | #[init]/#[destroy] |
| 应用启动 | SpringApplication.run() | ChimeraApplication.run() |
| 环境配置 | application.properties/yml | application.toml |
| Profile | @Profile | Profile 支持 |

## 运行测试

```bash
# 运行单元测试
cargo test -p chimera-core

# 运行综合示例
cargo run -p app-demo

# 带日志的运行
RUST_LOG=debug cargo run -p app-demo
```

## 项目结构

```
chimera/
├── chimera-core/       # 核心依赖注入容器
├── chimera-macros/     # 过程宏实现
└── examples/           # 示例代码
    └── app-demo/       # 综合示例
```

## 后续规划

- [ ] 添加 Web 框架集成（Actix-web、Axum）
- [ ] 添加容器启动性能分析工具
- [ ] 支持 AOP 切面编程
- [ ] 提供 Bean 工厂扩展机制

## 贡献

欢迎提交 Issue 和 Pull Request！

## 许可

MIT OR Apache-2.0

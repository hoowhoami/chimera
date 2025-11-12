# Chimera Examples

本目录包含 Chimera 框架的示例项目，展示框架的核心特性和最佳实践。

## 📦 示例列表

### 1. app-demo - 综合示例（推荐）

展示 Chimera 框架的核心特性和完整的应用开发流程。

**特性演示：**
- ✅ `@ConfigurationProperties` - 自动批量绑定配置
- ✅ `@Component` - 自动组件扫描和注册
- ✅ `@autowired` - 自动依赖注入
- ✅ 类型安全的配置管理
- ✅ 环境变量覆盖
- ✅ Spring Boot 风格的应用启动

**运行示例：**
```bash
# 基本运行
cargo run -p app-demo

# 测试环境变量覆盖
APP_SERVER_PORT=9000 cargo run -p app-demo
APP_DATABASE_HOST=prod-db cargo run -p app-demo
```

**代码结构：**
```rust
// 配置定义 - 自动绑定
#[derive(ConfigurationProperties, Debug, Clone)]
#[prefix("database")]
struct DatabaseConfig { ... }

// 业务服务 - 自动注入
#[derive(Component)]
struct DatabaseService {
    #[autowired]
    config: Arc<DatabaseConfig>,
}

// 启动应用 - 全自动
let context = ChimeraApplication::new("ChimeraDemo")
    .config_file("application.toml")
    .run()?;
```

---

### 2. config-properties-demo - 配置绑定示例

深入展示 `@ConfigurationProperties` 的各种特性和使用场景。

**特性演示：**
- ✅ 批量绑定配置到结构体
- ✅ 字段名自动转换（snake_case → kebab-case）
- ✅ 自定义配置键名 `#[config("custom-key")]`
- ✅ 配置前缀支持 `#[prefix("database")]`
- ✅ 支持多种类型（String, i32, bool, etc.）
- ✅ 可选字段支持（Option<T>）
- ✅ 自动注册为 Bean
- ✅ 支持依赖注入到 Component

**运行示例：**
```bash
# 基本运行
cargo run -p config-properties-demo

# 测试环境变量覆盖
APP_DATABASE_HOST=prod-db APP_SERVER_PORT=9000 cargo run -p config-properties-demo
```

**代码亮点：**
```rust
// 定义配置结构
#[derive(ConfigurationProperties, Debug, Clone)]
#[prefix("database")]
struct DatabaseProperties {
    host: String,
    port: i32,

    // 自定义配置键名
    #[config("max-connections")]
    max_connections: i32,

    // 自动转换：ssl_enabled -> ssl-enabled
    ssl_enabled: bool,
}

// 自动绑定并注册为 Bean
let context = ChimeraApplication::new("MyApp").run()?;

// 方式 1: 从容器获取
let db_config = context.get_bean_by_type::<DatabaseProperties>()?;

// 方式 2: 注入到 Component
#[derive(Component)]
struct MyService {
    #[autowired]
    db_config: Arc<DatabaseProperties>,
}
```

---

## 🚀 快速开始

推荐从 **app-demo** 开始学习，它展示了框架的完整功能：

```bash
# 1. 运行综合示例
cargo run -p app-demo

# 2. 深入了解配置绑定
cargo run -p config-properties-demo
```

## 📚 学习路径

1. **app-demo** - 了解整体架构和核心特性
2. **config-properties-demo** - 深入学习配置管理

## 🔗 相关文档

- [README.md](../README.md) - 框架文档
- [chimera-core](../chimera-core/) - 核心库
- [chimera-macros](../chimera-macros/) - 宏定义

## 💡 提示

所有示例都支持通过环境变量覆盖配置：

```bash
# 格式：APP_{SECTION}_{KEY}
APP_DATABASE_HOST=prod-db cargo run -p app-demo
APP_SERVER_PORT=9000 cargo run -p app-demo
```

配置优先级：**环境变量 > 配置文件 > 默认值**

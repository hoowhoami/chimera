# Logging Configuration

Chimera 提供了内置的日志配置功能，支持多种日志格式和级别。

## 基本使用

### 默认日志（从环境变量读取）

```rust
use chimera_core::prelude::*;

fn main() -> Result<()> {
    // 自动初始化日志，从 RUST_LOG 环境变量读取配置
    let context = ChimeraApplication::new("MyApp")
        .run()?;

    Ok(())
}
```

运行时配置：
```bash
# 设置日志级别
RUST_LOG=debug cargo run

# 设置特定模块的日志级别
RUST_LOG=my_app=debug,chimera_core=info cargo run
```

### 自定义日志配置

```rust
use chimera_core::prelude::*;

fn main() -> Result<()> {
    let context = ChimeraApplication::new("MyApp")
        .logging(LoggingConfig::new()
            .level(LogLevel::Debug)
            .format(LogFormat::Pretty)
            .show_target(true))
        .run()?;

    Ok(())
}
```

## 日志级别

- `LogLevel::Trace` - 最详细的日志
- `LogLevel::Debug` - 调试信息
- `LogLevel::Info` - 一般信息（默认）
- `LogLevel::Warn` - 警告信息
- `LogLevel::Error` - 错误信息

## 日志格式

- `LogFormat::Compact` - 紧凑格式（默认）
- `LogFormat::Full` - 完整格式（包含时间、级别、目标）
- `LogFormat::Json` - JSON 格式（适合日志收集）
- `LogFormat::Pretty` - 美化格式（适合开发调试）

## 配置选项

```rust
LoggingConfig::new()
    .level(LogLevel::Info)           // 设置日志级别
    .format(LogFormat::Compact)      // 设置日志格式
    .show_timestamp(true)            // 显示时间戳
    .show_target(false)              // 显示目标模块
    .filter("my_crate=debug".to_string())  // 自定义过滤器
```

## 环境变量配置

| 环境变量 | 说明 | 示例 |
|---------|------|------|
| `RUST_LOG` | 日志过滤器 | `RUST_LOG=debug` |
| `LOG_LEVEL` | 日志级别 | `LOG_LEVEL=debug` |
| `LOG_FORMAT` | 日志格式 | `LOG_FORMAT=pretty` |

## 示例

### 开发环境配置

```rust
let context = ChimeraApplication::new("MyApp")
    .logging(LoggingConfig::new()
        .level(LogLevel::Debug)
        .format(LogFormat::Pretty)
        .show_target(true))
    .run()?;
```

### 生产环境配置

```rust
let context = ChimeraApplication::new("MyApp")
    .logging(LoggingConfig::new()
        .level(LogLevel::Info)
        .format(LogFormat::Json))
    .run()?;
```

### 从环境变量读取（推荐）

```rust
// 使用默认配置，自动从环境变量读取
let context = ChimeraApplication::new("MyApp")
    .run()?;
```

运行时配置：
```bash
# 开发环境
RUST_LOG=debug LOG_FORMAT=pretty cargo run

# 生产环境
RUST_LOG=info LOG_FORMAT=json cargo run
```

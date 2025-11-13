# Chimera Web Demo

完整的 Web 应用示例，展示如何使用 Chimera Web 构建 RESTful API。

## 运行示例

```bash
# 从项目根目录运行
cargo run -p web-demo

# 或从示例目录运行
cd examples/web-demo
cargo run
```

## 测试 API

### 健康检查

```bash
curl http://127.0.0.1:8080/health
```

### 获取应用信息

```bash
curl http://127.0.0.1:8080/api/info
```

### 用户管理

```bash
# 获取所有用户
curl http://127.0.0.1:8080/api/users

# 获取单个用户
curl http://127.0.0.1:8080/api/users/1

# 创建用户
curl -X POST http://127.0.0.1:8080/api/users \
  -H "Content-Type: application/json" \
  -d '{"name":"Alice","email":"alice@example.com"}'
```

## 环境变量配置

```bash
# 自定义端口
WEB_SERVER_PORT=9000 cargo run -p web-demo

# 自定义主机
WEB_SERVER_HOST=0.0.0.0 cargo run -p web-demo

# 组合配置
WEB_SERVER_HOST=0.0.0.0 WEB_SERVER_PORT=9000 cargo run -p web-demo
```

## 功能展示

此示例展示了以下功能：

1. **依赖注入** - UserService 自动注入 AppConfig
2. **Bean 提取器** - 在路由处理器中直接注入服务
3. **自动配置** - ServerProperties 从配置文件和环境变量加载
4. **REST API** - 统一的响应格式
5. **中间件** - 请求日志记录
6. **路径参数** - 使用 Path 提取器
7. **JSON 请求** - 使用 Json 提取器

## 项目结构

```
web-demo/
├── Cargo.toml              # 项目依赖
├── application.toml        # 应用配置
└── src/
    └── main.rs            # 主程序
```

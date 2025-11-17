use chimera_core::prelude::*;
use chimera_core_macros::Component;
use chimera_web_macros::{Controller, controller, get_mapping};
use chimera_web::prelude::*;
use serde_json::json;

#[derive(Controller, Component, Clone)]
#[route("/demo")]
pub struct DemoController;

#[controller]
impl DemoController {
    /// GET /demo/guide
    #[get_mapping("/guide")]
    async fn guide(&self) -> impl IntoResponse {
        ResponseEntity::ok(json!({
            "title": "Chimera Web 参数注入完整指南",
            "description": "统一在 controller 方法中使用提取器进行参数注入",

            "philosophy": {
                "principle": "所有参数都通过提取器明确声明，直接在 controller 方法中使用",
                "benefits": [
                    "统一且清晰：参数来源一目了然",
                    "自动注册：无需手动注册路由",
                    "类型安全：编译时检查",
                    "Spring Boot 风格：完全符合 Spring Boot 使用习惯"
                ]
            },

            "available_extractors": {
                "request_body": {
                    "name": "RequestBody<T>",
                    "description": "从 JSON body 反序列化（类似 @RequestBody）",
                    "example": "RequestBody(user): RequestBody<CreateUserRequest>",
                    "spring_boot": "@RequestBody User user"
                },
                "path_variable": {
                    "name": "PathVariable<T>",
                    "description": "从路径参数提取（类似 @PathVariable），支持正则验证",
                    "example": "PathVariable(id): PathVariable<u32>",
                    "validation": "path.validate(r\"^[a-zA-Z0-9_]+$\")",
                    "spring_boot": "@PathVariable Long id"
                },
                "request_param": {
                    "name": "RequestParam<T>",
                    "description": "从 query 参数反序列化（类似 @RequestParam）",
                    "example": "RequestParam(query): RequestParam<SearchQuery>",
                    "spring_boot": "@RequestParam String name"
                },
                "form_data": {
                    "name": "FormData<T>",
                    "description": "从表单数据反序列化（支持 application/x-www-form-urlencoded 和 multipart/form-data）",
                    "example": "FormData(form): FormData<LoginForm>",
                    "spring_boot": "@ModelAttribute LoginForm form"
                },
                "request_headers": {
                    "name": "RequestHeaders",
                    "description": "提取所有 HTTP 请求头（类似 @RequestHeader）",
                    "example": "RequestHeaders(headers): RequestHeaders",
                    "spring_boot": "@RequestHeader HttpHeaders headers"
                }
            },

            "usage_examples": {
                "simple": {
                    "description": "获取单个用户",
                    "code": "#[get_mapping(\"/users/:id\")] async fn get_user(&self, PathVariable(id): PathVariable<u32>) -> impl IntoResponse"
                },
                "with_body": {
                    "description": "创建用户",
                    "code": "#[post_mapping(\"/users\")] async fn create_user(&self, RequestBody(req): RequestBody<CreateUserRequest>) -> impl IntoResponse"
                },
                "combined": {
                    "description": "更新用户（组合路径参数和请求体）",
                    "code": "#[put_mapping(\"/users/:id\")] async fn update_user(&self, PathVariable(id): PathVariable<u32>, RequestBody(req): RequestBody<UpdateRequest>) -> impl IntoResponse"
                },
                "complex": {
                    "description": "复杂操作（三种提取器组合）",
                    "code": "#[post_mapping(\"/users/:id/actions\")] async fn action(&self, PathVariable(id): PathVariable<u32>, RequestParam(params): RequestParam<Value>, RequestBody(body): RequestBody<Value>) -> impl IntoResponse"
                }
            },

            "comparison_with_spring_boot": {
                "spring_boot": {
                    "controller": "@RestController @RequestMapping(\"/api\")",
                    "field_injection": "@Autowired UserService userService",
                    "request_body": "@RequestBody User user",
                    "path_variable": "@PathVariable Long id",
                    "request_param": "@RequestParam String name"
                },
                "chimera": {
                    "controller": "#[derive(Controller)] #[route(\"/api\")]",
                    "field_injection": "#[autowired] user_service: Arc<UserService>",
                    "request_body": "RequestBody(user): RequestBody<User>",
                    "path_variable": "PathVariable(id): PathVariable<u32>",
                    "request_param": "RequestParam(name): RequestParam<String>"
                }
            },

            "key_features": [
                "完全自动化：路由自动注册，无需手动配置",
                "类型安全：所有参数都有明确的类型",
                "错误处理：提取失败自动返回适当的 HTTP 状态码",
                "灵活组合：可以在一个方法中使用多个提取器",
                "Spring Boot 风格：完全符合 Java 开发者的使用习惯"
            ]
        }))
    }
}

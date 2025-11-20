//! Multipart/form-data 支持
//!
//! 基于 multer 提供文件上传功能

use axum::{
    async_trait,
    extract::{FromRequest, Request},
    http::header::CONTENT_TYPE,
};
use bytes::Bytes;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

use crate::{constants::*, exception_handler::WebError};

pub use multer::{Field, Multipart as MulterMultipart};

/// Multipart 配置属性
///
/// 可通过配置文件的 `chimera.web.multipart` 前缀配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipartProperties {
    /// 最大文件大小（字节），默认 10MB
    #[serde(default = "default_max_file_size")]
    pub max_file_size: usize,

    /// 最大字段数量，默认 100
    #[serde(default = "default_max_fields")]
    pub max_fields: usize,
}

fn default_max_file_size() -> usize {
    10 * 1024 * 1024 // 10MB
}

fn default_max_fields() -> usize {
    100
}

impl Default for MultipartProperties {
    fn default() -> Self {
        Self {
            max_file_size: default_max_file_size(),
            max_fields: default_max_fields(),
        }
    }
}

impl MultipartProperties {
    /// 从 Environment 加载配置
    pub fn from_environment(env: &chimera_core::prelude::Environment) -> Self {
        Self {
            max_file_size: env
                .get_i64(MULTIPART_MAX_FILE_SIZE)
                .map(|v| v as usize)
                .unwrap_or_else(default_max_file_size),
            max_fields: env
                .get_i64(MULTIPART_MAX_FIELDS)
                .map(|v| v as usize)
                .unwrap_or_else(default_max_fields),
        }
    }

    /// 转换为 multer::Constraints
    pub fn to_multer_constraints(&self) -> multer::Constraints {
        multer::Constraints::new()
            .size_limit(multer::SizeLimit::new().whole_stream(self.max_file_size as u64))
    }
}

/// Multipart 提取器 - 用于手动处理 multipart/form-data 请求
pub struct Multipart {
    inner: MulterMultipart<'static>,
}

impl Multipart {
    pub fn new(inner: MulterMultipart<'static>) -> Self {
        Self { inner }
    }

    pub async fn next_field(&mut self) -> Result<Option<Field<'static>>, multer::Error> {
        self.inner.next_field().await
    }
}

#[async_trait]
impl<S> FromRequest<S> for Multipart
where
    S: Send + Sync,
{
    type Rejection = WebError;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let content_type = req
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| WebError::FormParse {
                message: "Missing Content-Type header".to_string(),
            })?;

        let boundary = multer::parse_boundary(content_type).map_err(|e| WebError::FormParse {
            message: format!("Failed to parse boundary: {}", e),
        })?;

        let config = req
            .extensions()
            .get::<Arc<MultipartProperties>>()
            .cloned()
            .unwrap_or_else(|| Arc::new(MultipartProperties::default()));

        let constraints = config.to_multer_constraints();

        let body = req.into_body();
        let body_stream = axum::body::to_bytes(body, usize::MAX)
            .await
            .map_err(|e| WebError::FormParse {
                message: format!("Failed to read request body: {}", e),
            })?;

        let multipart = MulterMultipart::with_constraints(
            futures_util::stream::once(async move { Ok::<Bytes, std::io::Error>(body_stream) }),
            boundary,
            constraints,
        );

        Ok(Multipart::new(multipart))
    }
}

/// 上传文件信息 - 类似 Spring 的 MultipartFile
#[derive(Debug, Clone, Default)]
pub struct MultipartFile {
    /// 字段名称
    pub field_name: String,

    /// 原始文件名（如果提供）
    pub filename: Option<String>,

    /// 文件内容类型（如果提供）
    pub content_type: Option<String>,

    /// 文件数据
    pub data: Bytes,
}

impl MultipartFile {
    /// 从 Field 创建 MultipartFile
    pub async fn from_field(field: Field<'_>) -> Result<Self, multer::Error> {
        let field_name = field.name().unwrap_or("unknown").to_string();
        let filename = field.file_name().map(|s| s.to_string());
        let content_type = field.content_type().map(|mime| mime.to_string());
        let data = field.bytes().await?;

        Ok(Self {
            field_name,
            filename,
            content_type,
            data,
        })
    }

    /// 获取文件大小（字节）
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// 判断是否为空文件
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// 获取文件扩展名
    pub fn extension(&self) -> Option<&str> {
        self.filename
            .as_ref()
            .and_then(|name| name.rfind('.').map(|pos| &name[pos + 1..]))
    }

    /// 将文件数据保存到指定路径
    pub async fn save_to(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        tokio::fs::write(path, &self.data).await
    }

    /// 获取文件数据的字节切片
    pub fn bytes(&self) -> &[u8] {
        &self.data
    }
}

/// Multipart 原始数据
#[derive(Debug, Clone, Default)]
pub struct MultipartRawData {
    /// 表单字段（非文件字段）
    pub fields: HashMap<String, String>,

    /// 上传的文件（按字段名分组）
    pub files: HashMap<String, Vec<MultipartFile>>,
}

impl MultipartRawData {
    /// 从 Multipart 提取原始数据
    pub async fn from_multipart(mut multipart: Multipart) -> Result<Self, WebError> {
        let mut fields = HashMap::new();
        let mut files: HashMap<String, Vec<MultipartFile>> = HashMap::new();

        while let Some(field) = multipart.next_field().await.map_err(|e| WebError::FormParse {
            message: format!("Failed to read multipart field: {}", e),
        })? {
            let field_name = field.name().unwrap_or("unknown").to_string();

            if field.file_name().is_some() {
                let file =
                    MultipartFile::from_field(field)
                        .await
                        .map_err(|e| WebError::FormParse {
                            message: format!("Failed to read file field '{}': {}", field_name, e),
                        })?;
                files.entry(field_name).or_default().push(file);
            } else {
                let value = field.text().await.map_err(|e| WebError::FormParse {
                    message: format!("Failed to read text field '{}': {}", field_name, e),
                })?;
                fields.insert(field_name, value);
            }
        }

        Ok(Self { fields, files })
    }

    /// 获取单个文件
    pub fn get_file(&mut self, name: &str) -> Option<MultipartFile> {
        self.files.get_mut(name).and_then(|v| {
            if !v.is_empty() {
                Some(v.remove(0))
            } else {
                None
            }
        })
    }

    /// 获取多个文件
    pub fn get_files(&mut self, name: &str) -> Vec<MultipartFile> {
        self.files.remove(name).unwrap_or_default()
    }
}

/// MultipartForm 提取器 - 类似 Spring Boot 的 @ModelAttribute
///
/// 自动将 multipart/form-data 映射到结构体
///
/// # 支持的字段类型
///
/// - `String`, `Option<String>` - 普通表单字段
/// - `i32`, `u64`, `bool` 等 - 可解析的表单字段
/// - `MultipartFile` - 单个文件上传
/// - `Option<MultipartFile>` - 可选文件上传
/// - `Vec<MultipartFile>` - 多文件上传
///
/// # 用法示例
///
/// ```ignore
/// use chimera_web::multipart::{MultipartForm, MultipartFile};
/// use serde::Deserialize;
/// use validator::Validate;
///
/// #[derive(Debug, Deserialize, Validate, FromMultipart)]
/// struct UploadForm {
///     #[validate(length(min = 1))]
///     title: String,
///
///     description: Option<String>,
///
///     #[multipart(file)]
///     avatar: MultipartFile,
///
///     #[multipart(file)]
///     documents: Vec<MultipartFile>,
/// }
///
/// #[post_mapping("/upload")]
/// async fn upload(MultipartForm(form): MultipartForm<UploadForm>) -> impl IntoResponse {
///     println!("Title: {}", form.title);
///     println!("Avatar size: {}", form.avatar.size());
///     ResponseEntity::ok("Success")
/// }
/// ```
pub struct MultipartForm<T>(pub T);

/// FromMultipart trait - 用于从 multipart 数据构建类型
pub trait FromMultipart: Sized {
    fn from_multipart(raw: MultipartRawData) -> Result<Self, WebError>;
}

#[async_trait]
impl<S, T> FromRequest<S> for MultipartForm<T>
where
    S: Send + Sync,
    T: FromMultipart,
{
    type Rejection = WebError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let multipart = Multipart::from_request(req, state).await?;
        let raw = MultipartRawData::from_multipart(multipart).await?;
        let data = T::from_multipart(raw)?;
        Ok(MultipartForm(data))
    }
}

/// ValidatedMultipartForm 提取器 - 带验证的 MultipartForm
///
/// 类似 ValidatedFormData，自动验证反序列化后的数据
pub struct ValidatedMultipartForm<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for ValidatedMultipartForm<T>
where
    S: Send + Sync,
    T: FromMultipart + validator::Validate,
{
    type Rejection = WebError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let multipart = Multipart::from_request(req, state).await?;
        let raw = MultipartRawData::from_multipart(multipart).await?;
        let data = T::from_multipart(raw)?;

        // 执行验证
        data.validate().map_err(|e: validator::ValidationErrors| {
            tracing::debug!(error = ?e, "Validation error");

            let mut field_errors = std::collections::HashMap::new();

            for (field_name, field_errors_vec) in e.field_errors() {
                let messages: Vec<String> = field_errors_vec
                    .iter()
                    .map(|error| {
                        error
                            .message
                            .as_ref()
                            .map(|cow| cow.to_string())
                            .unwrap_or_else(|| {
                                format!("Validation failed for field: {}", field_name)
                            })
                    })
                    .collect();

                if !messages.is_empty() {
                    field_errors.insert(field_name.to_string(), messages);
                }
            }

            WebError::Validation {
                message: "Validation failed".to_string(),
                field_errors: Some(field_errors),
            }
        })?;

        Ok(ValidatedMultipartForm(data))
    }
}

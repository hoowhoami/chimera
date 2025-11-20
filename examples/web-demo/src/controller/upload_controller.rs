use chimera_core::prelude::*;
use chimera_core_macros::{component, Component};
use chimera_web::multipart::{Multipart, MultipartFile, MultipartForm, ValidatedMultipartForm};
use chimera_web::prelude::*;
use chimera_web_macros::{controller, post_mapping, FromMultipart};
use validator::Validate;

/// 文件上传 Controller
///
/// 展示如何使用 Multipart 提取器处理文件上传
#[controller("/upload")]
#[derive(Component, Clone)]
pub struct UploadController {}

/// 单文件上传表单
#[derive(Debug, FromMultipart)]
struct SingleUploadForm {
    title: String,
    description: Option<String>,
    file: MultipartFile,
}

/// 多文件上传表单（带验证）
#[derive(Debug, Validate, FromMultipart)]
struct MultiUploadForm {
    #[validate(length(min = 1, max = 100, message = "标题长度必须在1-100个字符之间"))]
    title: String,

    #[validate(length(max = 500, message = "描述不能超过500个字符"))]
    description: Option<String>,

    avatar: MultipartFile,
    documents: Vec<MultipartFile>,
}

#[component]
#[controller]
impl UploadController {
    /// 单文件上传示例 - 使用 Multipart 手动处理
    #[post_mapping("/manual")]
    async fn upload_manual(&self, multipart: Multipart) -> impl IntoResponse {
        let mut multipart = multipart;
        let mut uploaded_files = Vec::new();

        while let Some(field) = multipart.next_field().await.unwrap() {
            let name = field.name().unwrap_or("unknown").to_string();
            let filename = field.file_name().map(|s| s.to_string());
            let content_type = field.content_type().map(|m| m.to_string());

            let data = field.bytes().await.unwrap();

            uploaded_files.push(serde_json::json!({
                "field_name": name,
                "filename": filename,
                "content_type": content_type,
                "size": data.len(),
            }));
        }

        ResponseEntity::ok(serde_json::json!({
            "success": true,
            "message": "Files uploaded successfully",
            "files": uploaded_files
        }))
    }

    /// 单文件上传示例 - 使用 MultipartForm 自动映射
    #[post_mapping("/single")]
    async fn upload_single(&self, MultipartForm(form): MultipartForm<SingleUploadForm>) -> impl IntoResponse {
        ResponseEntity::ok(serde_json::json!({
            "success": true,
            "message": "File uploaded successfully",
            "data": {
                "title": form.title,
                "description": form.description,
                "file": {
                    "filename": form.file.filename,
                    "content_type": form.file.content_type,
                    "size": form.file.size(),
                    "extension": form.file.extension(),
                }
            }
        }))
    }

    /// 多文件上传示例 - 带验证
    #[post_mapping("/multiple")]
    async fn upload_multiple(
        &self,
        ValidatedMultipartForm(form): ValidatedMultipartForm<MultiUploadForm>,
    ) -> impl IntoResponse {
        let documents_info: Vec<_> = form
            .documents
            .iter()
            .map(|file| {
                serde_json::json!({
                    "filename": file.filename,
                    "content_type": file.content_type,
                    "size": file.size(),
                    "extension": file.extension(),
                })
            })
            .collect();

        ResponseEntity::ok(serde_json::json!({
            "success": true,
            "message": format!("Uploaded {} file(s)", form.documents.len() + 1),
            "data": {
                "title": form.title,
                "description": form.description,
                "avatar": {
                    "filename": form.avatar.filename,
                    "size": form.avatar.size(),
                },
                "documents": documents_info,
            }
        }))
    }

    /// 文件上传并保存到磁盘
    #[post_mapping("/save")]
    async fn upload_and_save(
        &self,
        ValidatedMultipartForm(form): ValidatedMultipartForm<MultiUploadForm>,
    ) -> impl IntoResponse {
        let upload_dir = std::path::Path::new("./uploads");

        // 确保上传目录存在
        if let Err(e) = tokio::fs::create_dir_all(upload_dir).await {
            return ResponseEntity::internal_error(serde_json::json!({
                "success": false,
                "message": format!("Failed to create upload directory: {}", e),
            }));
        }

        let mut saved_files = Vec::new();

        // 保存头像
        if let Some(filename) = &form.avatar.filename {
            let file_path = upload_dir.join(filename);
            match form.avatar.save_to(&file_path).await {
                Ok(_) => {
                    saved_files.push(serde_json::json!({
                        "type": "avatar",
                        "filename": filename,
                        "path": file_path.to_string_lossy(),
                        "size": form.avatar.size(),
                    }));
                }
                Err(e) => {
                    return ResponseEntity::internal_error(serde_json::json!({
                        "success": false,
                        "message": format!("Failed to save avatar: {}", e),
                    }));
                }
            }
        }

        // 保存文档
        for file in &form.documents {
            if let Some(filename) = &file.filename {
                let file_path = upload_dir.join(filename);
                match file.save_to(&file_path).await {
                    Ok(_) => {
                        saved_files.push(serde_json::json!({
                            "type": "document",
                            "filename": filename,
                            "path": file_path.to_string_lossy(),
                            "size": file.size(),
                        }));
                    }
                    Err(e) => {
                        return ResponseEntity::internal_error(serde_json::json!({
                            "success": false,
                            "message": format!("Failed to save document {}: {}", filename, e),
                        }));
                    }
                }
            }
        }

        ResponseEntity::ok(serde_json::json!({
            "success": true,
            "message": format!("Saved {} file(s)", saved_files.len()),
            "data": {
                "title": form.title,
                "description": form.description,
                "files": saved_files,
            }
        }))
    }
}

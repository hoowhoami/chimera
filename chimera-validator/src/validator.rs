use crate::error::{ValidationError, ValidationResult};
use regex::Regex;
use std::collections::HashMap;

/// 验证器 trait
pub trait Validate {
    fn validate(&self) -> ValidationResult<()>;
}

/// 验证规则
pub struct ValidationRules;

impl ValidationRules {
    /// 验证非空（Option）
    pub fn not_null<T>(value: &Option<T>, field: &str) -> ValidationResult<()> {
        if value.is_none() {
            return Err(ValidationError::field_error(
                field,
                format!("{} must not be null", field),
            ));
        }
        Ok(())
    }
    
    /// 验证字符串非空
    pub fn not_empty(value: &str, field: &str) -> ValidationResult<()> {
        if value.is_empty() {
            return Err(ValidationError::field_error(
                field,
                format!("{} must not be empty", field),
            ));
        }
        Ok(())
    }
    
    /// 验证字符串非空白
    pub fn not_blank(value: &str, field: &str) -> ValidationResult<()> {
        if value.trim().is_empty() {
            return Err(ValidationError::field_error(
                field,
                format!("{} must not be blank", field),
            ));
        }
        Ok(())
    }
    
    /// 验证字符串长度
    pub fn length(value: &str, field: &str, min: Option<usize>, max: Option<usize>) -> ValidationResult<()> {
        let len = value.len();
        
        if let Some(min_len) = min {
            if len < min_len {
                return Err(ValidationError::field_error(
                    field,
                    format!("{} length must be at least {}, but was {}", field, min_len, len),
                ));
            }
        }
        
        if let Some(max_len) = max {
            if len > max_len {
                return Err(ValidationError::field_error(
                    field,
                    format!("{} length must be at most {}, but was {}", field, max_len, len),
                ));
            }
        }
        
        Ok(())
    }
    
    /// 验证数值范围
    pub fn range<T: PartialOrd + std::fmt::Display>(
        value: T,
        field: &str,
        min: Option<T>,
        max: Option<T>,
    ) -> ValidationResult<()> {
        if let Some(min_val) = min {
            if value < min_val {
                return Err(ValidationError::field_error(
                    field,
                    format!("{} must be at least {}, but was {}", field, min_val, value),
                ));
            }
        }
        
        if let Some(max_val) = max {
            if value > max_val {
                return Err(ValidationError::field_error(
                    field,
                    format!("{} must be at most {}, but was {}", field, max_val, value),
                ));
            }
        }
        
        Ok(())
    }
    
    /// 验证邮箱格式
    pub fn email(value: &str, field: &str) -> ValidationResult<()> {
        let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
            .unwrap();
        
        if !email_regex.is_match(value) {
            return Err(ValidationError::field_error(
                field,
                format!("{} must be a valid email address", field),
            ));
        }
        
        Ok(())
    }
    
    /// 验证正则表达式
    pub fn pattern(value: &str, field: &str, pattern: &str) -> ValidationResult<()> {
        let regex = Regex::new(pattern)
            .map_err(|e| ValidationError::new(format!("Invalid regex pattern: {}", e)))?;
        
        if !regex.is_match(value) {
            return Err(ValidationError::field_error(
                field,
                format!("{} must match pattern: {}", field, pattern),
            ));
        }
        
        Ok(())
    }
    
    /// 验证集合大小
    pub fn size<T>(value: &[T], field: &str, min: Option<usize>, max: Option<usize>) -> ValidationResult<()> {
        let len = value.len();
        
        if let Some(min_size) = min {
            if len < min_size {
                return Err(ValidationError::field_error(
                    field,
                    format!("{} size must be at least {}, but was {}", field, min_size, len),
                ));
            }
        }
        
        if let Some(max_size) = max {
            if len > max_size {
                return Err(ValidationError::field_error(
                    field,
                    format!("{} size must be at most {}, but was {}", field, max_size, len),
                ));
            }
        }
        
        Ok(())
    }
}

/// 验证器构建器
pub struct ValidatorBuilder {
    errors: HashMap<String, Vec<String>>,
}

impl ValidatorBuilder {
    pub fn new() -> Self {
        Self {
            errors: HashMap::new(),
        }
    }
    
    pub fn add_error(&mut self, field: impl Into<String>, message: impl Into<String>) {
        self.errors
            .entry(field.into())
            .or_insert_with(Vec::new)
            .push(message.into());
    }
    
    pub fn add_result(&mut self, result: ValidationResult<()>) {
        if let Err(ValidationError::FieldErrors(errors)) = result {
            for (field, messages) in errors {
                self.errors
                    .entry(field)
                    .or_insert_with(Vec::new)
                    .extend(messages);
            }
        }
    }
    
    pub fn build(self) -> ValidationResult<()> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationError::FieldErrors(self.errors))
        }
    }
}

impl Default for ValidatorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

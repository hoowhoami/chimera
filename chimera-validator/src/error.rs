use std::collections::HashMap;
use thiserror::Error;

/// 验证错误
#[derive(Debug, Error, Clone)]
pub enum ValidationError {
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
    
    #[error("Field validation errors")]
    FieldErrors(HashMap<String, Vec<String>>),
}

impl ValidationError {
    pub fn new(message: impl Into<String>) -> Self {
        Self::ValidationFailed(message.into())
    }
    
    pub fn field_error(field: impl Into<String>, message: impl Into<String>) -> Self {
        let mut errors = HashMap::new();
        errors.insert(field.into(), vec![message.into()]);
        Self::FieldErrors(errors)
    }
    
    pub fn add_field_error(&mut self, field: impl Into<String>, message: impl Into<String>) {
        match self {
            Self::FieldErrors(errors) => {
                errors.entry(field.into())
                    .or_insert_with(Vec::new)
                    .push(message.into());
            }
            _ => {}
        }
    }
    
    pub fn merge(&mut self, other: ValidationError) {
        match (self, other) {
            (Self::FieldErrors(errors1), Self::FieldErrors(errors2)) => {
                for (field, messages) in errors2 {
                    errors1.entry(field)
                        .or_insert_with(Vec::new)
                        .extend(messages);
                }
            }
            _ => {}
        }
    }
}

pub type ValidationResult<T> = Result<T, ValidationError>;

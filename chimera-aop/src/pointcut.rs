//! 切点（Pointcut）表达式系统
//!
//! 定义了如何匹配连接点的规则

use crate::JoinPoint;
use regex::Regex;
use std::sync::Arc;

/// 切点表达式
///
/// 用于匹配连接点
#[derive(Clone)]
pub enum PointcutExpression {
    /// 匹配所有方法
    All,

    /// 匹配特定类型的所有方法
    /// 例如：TypePattern("UserService")
    TypePattern(String),

    /// 匹配特定方法名
    /// 例如：MethodPattern("get_user")
    MethodPattern(String),

    /// 匹配特定类型的特定方法
    /// 例如：execution(* UserService.get_user(..))
    Execution {
        type_pattern: String,
        method_pattern: String,
    },

    /// 使用正则表达式匹配类型
    TypeRegex(Regex),

    /// 使用正则表达式匹配方法
    MethodRegex(Regex),

    /// 自定义匹配函数
    Custom(Arc<dyn Fn(&JoinPoint) -> bool + Send + Sync>),

    /// 与运算（AND）
    And(Box<PointcutExpression>, Box<PointcutExpression>),

    /// 或运算（OR）
    Or(Box<PointcutExpression>, Box<PointcutExpression>),

    /// 非运算（NOT）
    Not(Box<PointcutExpression>),
}

impl PointcutExpression {
    /// 检查连接点是否匹配
    pub fn matches(&self, join_point: &JoinPoint) -> bool {
        match self {
            PointcutExpression::All => true,

            PointcutExpression::TypePattern(pattern) => {
                Self::pattern_matches(pattern, join_point.target_type)
            }

            PointcutExpression::MethodPattern(pattern) => {
                Self::pattern_matches(pattern, join_point.method_name)
            }

            PointcutExpression::Execution {
                type_pattern,
                method_pattern,
            } => {
                Self::pattern_matches(type_pattern, join_point.target_type)
                    && Self::pattern_matches(method_pattern, join_point.method_name)
            }

            PointcutExpression::TypeRegex(regex) => regex.is_match(join_point.target_type),

            PointcutExpression::MethodRegex(regex) => regex.is_match(join_point.method_name),

            PointcutExpression::Custom(func) => func(join_point),

            PointcutExpression::And(left, right) => {
                left.matches(join_point) && right.matches(join_point)
            }

            PointcutExpression::Or(left, right) => {
                left.matches(join_point) || right.matches(join_point)
            }

            PointcutExpression::Not(expr) => !expr.matches(join_point),
        }
    }

    /// 简单的模式匹配（支持 * 通配符）
    ///
    /// 支持的模式：
    /// - `*` - 匹配任意字符串
    /// - `User*` - 以 User 开头
    /// - `*Service` - 以 Service 结尾
    /// - `*Service*` - 包含 Service
    fn pattern_matches(pattern: &str, target: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if !pattern.contains('*') {
            return pattern == target;
        }

        // 将 * 转换为正则表达式
        let regex_pattern = pattern.replace('*', ".*");
        let regex_pattern = format!("^{}$", regex_pattern);

        if let Ok(regex) = Regex::new(&regex_pattern) {
            regex.is_match(target)
        } else {
            false
        }
    }

    /// 创建 execution 表达式
    ///
    /// 例如：execution("* UserService.get_user(..)")
    /// 格式：返回类型 类型名.方法名(参数)
    ///
    /// 简化版本，只支持类型和方法名匹配
    pub fn execution(expression: &str) -> Self {
        // 解析表达式: "* UserService.get_user(..)"
        let parts: Vec<&str> = expression.split_whitespace().collect();

        if parts.len() < 2 {
            return PointcutExpression::All;
        }

        let method_part = parts[1];
        if let Some((type_pattern, method_pattern)) = method_part.split_once('.') {
            // 移除参数部分 "(..)"
            let method_pattern = method_pattern.trim_end_matches("(..)");

            PointcutExpression::Execution {
                type_pattern: type_pattern.to_string(),
                method_pattern: method_pattern.to_string(),
            }
        } else {
            PointcutExpression::MethodPattern(method_part.to_string())
        }
    }

    /// 与运算
    pub fn and(self, other: PointcutExpression) -> Self {
        PointcutExpression::And(Box::new(self), Box::new(other))
    }

    /// 或运算
    pub fn or(self, other: PointcutExpression) -> Self {
        PointcutExpression::Or(Box::new(self), Box::new(other))
    }

    /// 非运算
    pub fn not(self) -> Self {
        PointcutExpression::Not(Box::new(self))
    }
}

impl std::fmt::Debug for PointcutExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PointcutExpression::All => write!(f, "All"),
            PointcutExpression::TypePattern(p) => write!(f, "TypePattern({})", p),
            PointcutExpression::MethodPattern(p) => write!(f, "MethodPattern({})", p),
            PointcutExpression::Execution { type_pattern, method_pattern } => {
                write!(f, "Execution({}.{})", type_pattern, method_pattern)
            }
            PointcutExpression::TypeRegex(_) => write!(f, "TypeRegex(...)"),
            PointcutExpression::MethodRegex(_) => write!(f, "MethodRegex(...)"),
            PointcutExpression::Custom(_) => write!(f, "Custom(...)"),
            PointcutExpression::And(l, r) => write!(f, "And({:?}, {:?})", l, r),
            PointcutExpression::Or(l, r) => write!(f, "Or({:?}, {:?})", l, r),
            PointcutExpression::Not(e) => write!(f, "Not({:?})", e),
        }
    }
}

/// 切点 Trait
///
/// 定义切点的行为
pub trait Pointcut: Send + Sync {
    /// 获取切点表达式
    fn expression(&self) -> &PointcutExpression;

    /// 检查是否匹配
    fn matches(&self, join_point: &JoinPoint) -> bool {
        self.expression().matches(join_point)
    }
}

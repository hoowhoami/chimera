use chimera_core::prelude::*;
use chimera_core_macros::Component;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::models::{SearchQuery, User};

#[derive(Component, Clone)]
#[bean("userService")]
pub struct UserService {
    #[autowired]
    _config: Arc<AppConfig>,
}

impl UserService {
    pub fn list_users(&self) -> Vec<User> {
        vec![
            User {
                id: 1,
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
            },
            User {
                id: 2,
                name: "Bob".to_string(),
                email: "bob@example.com".to_string(),
            },
        ]
    }

    pub fn get_user_by_id(&self, id: u32) -> Option<User> {
        self.list_users().into_iter().find(|u| u.id == id)
    }

    pub fn search_users(&self, query: SearchQuery) -> Vec<User> {
        let mut users = self.list_users();

        if let Some(name) = query.name {
            users.retain(|u| u.name.contains(&name));
        }
        if let Some(email) = query.email {
            users.retain(|u| u.email.contains(&email));
        }

        users
    }
}

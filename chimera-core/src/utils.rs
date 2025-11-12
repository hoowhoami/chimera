//! Utility functions for the container
//!
//! This module provides common utility functions used throughout the crate,
//! following Rust best practices for naming conventions and string manipulation.

/// Naming convention utilities for bean names
pub mod naming {
    /// Converts a PascalCase type name to camelCase for bean naming.
    ///
    /// This is the default bean naming strategy, similar to Spring's behavior
    /// where `UserService` becomes `userService`.
    ///
    /// # Examples
    ///
    /// ```
    /// use chimera_core::utils::naming::to_camel_case;
    ///
    /// assert_eq!(to_camel_case("UserService"), "userService");
    /// assert_eq!(to_camel_case("DatabaseConnectionPool"), "databaseConnectionPool");
    /// assert_eq!(to_camel_case("A"), "a");
    /// assert_eq!(to_camel_case(""), "");
    /// ```
    ///
    /// # Performance
    ///
    /// This function allocates a new String only when necessary. For empty strings,
    /// it returns an empty String without allocating additional memory.
    pub fn to_camel_case(s: &str) -> String {
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => {
                let mut result = String::with_capacity(s.len());
                result.extend(first.to_lowercase());
                result.push_str(chars.as_str());
                result
            }
        }
    }

    /// Converts a string to snake_case.
    ///
    /// This is useful for configuration keys and other lowercase identifiers.
    ///
    /// # Examples
    ///
    /// ```
    /// use chimera_core::utils::naming::to_snake_case;
    ///
    /// assert_eq!(to_snake_case("UserService"), "user_service");
    /// assert_eq!(to_snake_case("DatabaseConnectionPool"), "database_connection_pool");
    /// assert_eq!(to_snake_case("HTTPServer"), "h_t_t_p_server");
    /// ```
    pub fn to_snake_case(s: &str) -> String {
        let mut result = String::with_capacity(s.len() + s.len() / 2);
        let mut chars = s.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch.is_uppercase() {
                if !result.is_empty() {
                    result.push('_');
                }
                result.extend(ch.to_lowercase());
            } else {
                result.push(ch);
            }
        }

        result
    }

    /// Converts a string to kebab-case.
    ///
    /// This is useful for configuration file names and URLs.
    ///
    /// # Examples
    ///
    /// ```
    /// use chimera_core::utils::naming::to_kebab_case;
    ///
    /// assert_eq!(to_kebab_case("UserService"), "user-service");
    /// assert_eq!(to_kebab_case("DatabaseConnectionPool"), "database-connection-pool");
    /// ```
    pub fn to_kebab_case(s: &str) -> String {
        to_snake_case(s).replace('_', "-")
    }
}

/// Dependency resolution utilities
pub mod dependency {
    use std::collections::{HashSet, HashMap};

    /// Tracks beans currently being created to detect circular dependencies.
    ///
    /// This is a thread-safe wrapper around a HashSet that maintains the set
    /// of beans currently in the creation process.
    #[derive(Debug, Default)]
    pub struct CreationTracker {
        creating: std::sync::RwLock<HashSet<String>>,
    }

    impl CreationTracker {
        /// Creates a new empty creation tracker.
        pub fn new() -> Self {
            Self {
                creating: std::sync::RwLock::new(HashSet::new()),
            }
        }

        /// Checks if a bean is currently being created.
        ///
        /// # Errors
        ///
        /// Returns an error if the lock is poisoned.
        pub fn is_creating(&self, name: &str) -> Result<bool, String> {
            self.creating
                .read()
                .map(|guard| guard.contains(name))
                .map_err(|e| format!("Failed to acquire read lock: {}", e))
        }

        /// Marks a bean as being created.
        ///
        /// Returns `true` if the bean was not already being created,
        /// `false` if it was already in the creating set (circular dependency detected).
        ///
        /// # Errors
        ///
        /// Returns an error if the lock is poisoned.
        pub fn start_creating(&self, name: &str) -> Result<bool, String> {
            self.creating
                .write()
                .map(|mut guard| guard.insert(name.to_string()))
                .map_err(|e| format!("Failed to acquire write lock: {}", e))
        }

        /// Marks a bean as finished being created.
        ///
        /// # Errors
        ///
        /// Returns an error if the lock is poisoned.
        pub fn finish_creating(&self, name: &str) -> Result<(), String> {
            self.creating
                .write()
                .map(|mut guard| {
                    guard.remove(name);
                })
                .map_err(|e| format!("Failed to acquire write lock: {}", e))
        }

        /// Gets a snapshot of all beans currently being created.
        ///
        /// This is useful for debugging and error messages.
        ///
        /// # Errors
        ///
        /// Returns an error if the lock is poisoned.
        pub fn current_creating(&self) -> Result<Vec<String>, String> {
            self.creating
                .read()
                .map(|guard| guard.iter().cloned().collect())
                .map_err(|e| format!("Failed to acquire read lock: {}", e))
        }
    }

    /// Dependency graph analysis result
    #[derive(Debug)]
    pub enum DependencyValidationError {
        /// Circular dependency detected
        CircularDependency {
            /// The dependency chain forming the cycle
            cycle: Vec<String>,
        },
        /// Missing dependency detected
        MissingDependency {
            /// The bean that requires the dependency
            bean: String,
            /// The missing dependency
            missing: String,
        },
    }

    impl std::fmt::Display for DependencyValidationError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::CircularDependency { cycle } => {
                    write!(f, "Circular dependency detected: {}", cycle.join(" -> "))
                }
                Self::MissingDependency { bean, missing } => {
                    write!(f, "Bean '{}' depends on '{}' which is not registered", bean, missing)
                }
            }
        }
    }

    /// Validates dependency graph for circular dependencies and missing beans
    ///
    /// # Arguments
    ///
    /// * `dependencies` - Map of bean name to its list of dependencies
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if no issues found, or Err with the first detected issue
    pub fn validate_dependency_graph(
        dependencies: &HashMap<String, Vec<String>>,
    ) -> Result<(), DependencyValidationError> {
        // Check for missing dependencies
        for (bean_name, deps) in dependencies {
            for dep in deps {
                if !dependencies.contains_key(dep) {
                    return Err(DependencyValidationError::MissingDependency {
                        bean: bean_name.clone(),
                        missing: dep.clone(),
                    });
                }
            }
        }

        // Check for circular dependencies using DFS
        let mut visited = HashSet::new();
        let mut rec_stack = Vec::new();

        for bean_name in dependencies.keys() {
            if !visited.contains(bean_name) {
                if let Some(cycle) = detect_cycle_dfs(
                    bean_name,
                    dependencies,
                    &mut visited,
                    &mut rec_stack,
                ) {
                    return Err(DependencyValidationError::CircularDependency { cycle });
                }
            }
        }

        Ok(())
    }

    /// DFS-based cycle detection
    ///
    /// Returns Some(cycle) if a cycle is detected, None otherwise
    fn detect_cycle_dfs(
        node: &str,
        graph: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        rec_stack: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(node.to_string());
        rec_stack.push(node.to_string());

        if let Some(deps) = graph.get(node) {
            for dep in deps {
                if !visited.contains(dep) {
                    if let Some(cycle) = detect_cycle_dfs(dep, graph, visited, rec_stack) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(dep) {
                    // Found a cycle
                    let start_idx = rec_stack.iter().position(|x| x == dep).unwrap();
                    let mut cycle = rec_stack[start_idx..].to_vec();
                    cycle.push(dep.to_string());
                    return Some(cycle);
                }
            }
        }

        rec_stack.pop();
        None
    }

    /// Performs topological sort on dependency graph
    ///
    /// Returns a vector of bean names in dependency order (dependencies before dependents)
    ///
    /// # Arguments
    ///
    /// * `dependencies` - Map of bean name to its list of dependencies
    ///
    /// # Returns
    ///
    /// Returns Ok(sorted_beans) if successful, or Err(error_message) if there's a circular dependency
    pub fn topological_sort(
        dependencies: &HashMap<String, Vec<String>>,
    ) -> Result<Vec<String>, String> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();

        // Initialize in-degree and build dependency graph
        // For each bean -> [deps], we want deps to come before bean
        // So we add edges from deps to bean
        for (bean, deps) in dependencies {
            in_degree.entry(bean.clone()).or_insert(0);
            *in_degree.get_mut(bean).unwrap() += deps.len();

            for dep in deps {
                in_degree.entry(dep.clone()).or_insert(0);
                graph.entry(dep.clone()).or_insert_with(Vec::new).push(bean.clone());
            }
        }

        // Collect nodes with no incoming edges
        let mut queue: Vec<String> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(bean, _)| bean.clone())
            .collect();

        let mut result = Vec::new();

        while let Some(node) = queue.pop() {
            result.push(node.clone());

            if let Some(dependents) = graph.get(&node) {
                for dependent in dependents {
                    if let Some(degree) = in_degree.get_mut(dependent) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push(dependent.clone());
                        }
                    }
                }
            }
        }

        // Check if all nodes were processed
        if result.len() != in_degree.len() {
            return Err("Circular dependency detected during topological sort".to_string());
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    mod naming_tests {
        use super::super::naming::*;

        #[test]
        fn test_to_camel_case() {
            assert_eq!(to_camel_case("UserService"), "userService");
            assert_eq!(to_camel_case("DatabaseService"), "databaseService");
            assert_eq!(to_camel_case("A"), "a");
            assert_eq!(to_camel_case("AB"), "aB");
            assert_eq!(to_camel_case(""), "");
            assert_eq!(to_camel_case("lowerCase"), "lowerCase");
        }

        #[test]
        fn test_to_snake_case() {
            assert_eq!(to_snake_case("UserService"), "user_service");
            assert_eq!(to_snake_case("DatabaseConnectionPool"), "database_connection_pool");
            assert_eq!(to_snake_case("HTTPServer"), "h_t_t_p_server");
            assert_eq!(to_snake_case(""), "");
            assert_eq!(to_snake_case("lowercase"), "lowercase");
        }

        #[test]
        fn test_to_kebab_case() {
            assert_eq!(to_kebab_case("UserService"), "user-service");
            assert_eq!(to_kebab_case("DatabaseConnectionPool"), "database-connection-pool");
            assert_eq!(to_kebab_case(""), "");
        }
    }

    mod dependency_tests {
        use super::super::dependency::*;
        use std::collections::HashMap;

        #[test]
        fn test_creation_tracker() {
            let tracker = CreationTracker::new();

            // Initially nothing is being created
            assert_eq!(tracker.is_creating("serviceA").unwrap(), false);

            // Start creating serviceA
            assert_eq!(tracker.start_creating("serviceA").unwrap(), true);
            assert_eq!(tracker.is_creating("serviceA").unwrap(), true);

            // Try to start creating serviceA again (circular dependency)
            assert_eq!(tracker.start_creating("serviceA").unwrap(), false);

            // Finish creating serviceA
            tracker.finish_creating("serviceA").unwrap();
            assert_eq!(tracker.is_creating("serviceA").unwrap(), false);
        }

        #[test]
        fn test_current_creating() {
            let tracker = CreationTracker::new();

            tracker.start_creating("serviceA").unwrap();
            tracker.start_creating("serviceB").unwrap();

            let creating = tracker.current_creating().unwrap();
            assert_eq!(creating.len(), 2);
            assert!(creating.contains(&"serviceA".to_string()));
            assert!(creating.contains(&"serviceB".to_string()));
        }

        #[test]
        fn test_validate_missing_dependency() {
            let mut deps = HashMap::new();
            deps.insert("serviceA".to_string(), vec!["serviceB".to_string()]);
            // serviceB is not registered

            let result = validate_dependency_graph(&deps);
            assert!(result.is_err());

            if let Err(DependencyValidationError::MissingDependency { bean, missing }) = result {
                assert_eq!(bean, "serviceA");
                assert_eq!(missing, "serviceB");
            } else {
                panic!("Expected MissingDependency error");
            }
        }

        #[test]
        fn test_validate_circular_dependency() {
            let mut deps = HashMap::new();
            deps.insert("serviceA".to_string(), vec!["serviceB".to_string()]);
            deps.insert("serviceB".to_string(), vec!["serviceC".to_string()]);
            deps.insert("serviceC".to_string(), vec!["serviceA".to_string()]);

            let result = validate_dependency_graph(&deps);
            assert!(result.is_err());

            if let Err(DependencyValidationError::CircularDependency { cycle }) = result {
                assert!(cycle.len() >= 3);
                // The cycle should contain all three services
                let cycle_str = cycle.join(" -> ");
                assert!(cycle_str.contains("serviceA"));
                assert!(cycle_str.contains("serviceB"));
                assert!(cycle_str.contains("serviceC"));
            } else {
                panic!("Expected CircularDependency error");
            }
        }

        #[test]
        fn test_validate_valid_graph() {
            let mut deps = HashMap::new();
            deps.insert("config".to_string(), vec![]);
            deps.insert("database".to_string(), vec!["config".to_string()]);
            deps.insert("userService".to_string(), vec!["database".to_string(), "config".to_string()]);

            let result = validate_dependency_graph(&deps);
            assert!(result.is_ok());
        }

        #[test]
        fn test_validate_self_dependency() {
            let mut deps = HashMap::new();
            deps.insert("serviceA".to_string(), vec!["serviceA".to_string()]);

            let result = validate_dependency_graph(&deps);
            assert!(result.is_err());

            if let Err(DependencyValidationError::CircularDependency { cycle }) = result {
                assert_eq!(cycle.len(), 2); // serviceA -> serviceA
            } else {
                panic!("Expected CircularDependency error");
            }
        }
    }
}

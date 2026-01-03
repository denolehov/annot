/// A simple greeting module for testing annotation UI.
pub mod greeting {
    use std::fmt;

    /// Represents a greeting with a name and optional title.
    #[derive(Debug, Clone)]
    pub struct Greeting {
        name: String,
        title: Option<String>,
    }

    impl Greeting {
        /// Creates a new greeting for the given name.
        pub fn new(name: impl Into<String>) -> Self {
            Self {
                name: name.into(),
                title: None,
            }
        }

        /// Adds a title to the greeting.
        pub fn with_title(mut self, title: impl Into<String>) -> Self {
            self.title = Some(title.into());
            self
        }

        /// Formats the greeting as a string.
        pub fn format(&self) -> String {
            match &self.title {
                Some(title) => format!("Hello, {} {}!", title, self.name),
                None => format!("Hello, {}!", self.name),
            }
        }
    }

    impl fmt::Display for Greeting {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.format())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::greeting::Greeting;

    #[test]
    fn test_simple_greeting() {
        let g = Greeting::new("World");
        assert_eq!(g.format(), "Hello, World!");
    }

    #[test]
    fn test_greeting_with_title() {
        let g = Greeting::new("Smith").with_title("Dr.");
        assert_eq!(g.format(), "Hello, Dr. Smith!");
    }
}

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    pub url: String,
    pub runs_limit: Option<usize>,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:3000/graphql".to_string(),
            runs_limit: Some(20),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub last_context: String,
    pub contexts: HashMap<String, ContextConfig>,
}

impl Default for Config {
    fn default() -> Self {
        let mut contexts = HashMap::new();
        contexts.insert("default".to_string(), ContextConfig::default());

        Self {
            last_context: "default".to_string(),
            contexts,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            let config = Config::default();
            config.save()?;
            return Ok(config);
        }

        let config_str = fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&config_str)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        let config_path = Self::config_path()?;

        // Create directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let config_str = toml::to_string_pretty(self)?;
        fs::write(config_path, config_str)?;
        Ok(())
    }

    fn config_path() -> Result<PathBuf, Box<dyn Error>> {
        let home = dirs::home_dir().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Home directory not found")
        })?;

        Ok(home.join(".config").join("d9s").join("config.toml"))
    }

    pub fn get_current_context(&self) -> ContextConfig {
        // First try to get the last_context
        if let Some(context) = self.contexts.get(&self.last_context) {
            return context.clone();
        }

        // If last_context doesn't exist, try to use "default"
        if let Some(context) = self.contexts.get("default") {
            return context.clone();
        }

        // If all else fails, return a default config
        ContextConfig::default()
    }
    pub fn set_context(&mut self, name: &str) -> Result<(), Box<dyn Error>> {
        if !self.contexts.contains_key(name) {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Context '{}' not found", name),
            )));
        }

        self.last_context = name.to_string();
        self.save()?;
        Ok(())
    }

    pub fn add_context(&mut self, name: &str, config: ContextConfig) -> Result<(), Box<dyn Error>> {
        self.contexts.insert(name.to_string(), config);
        self.save()?;
        Ok(())
    }

    pub fn remove_context(&mut self, name: &str) -> Result<(), Box<dyn Error>> {
        if name == "default" {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Cannot remove default context",
            )));
        }

        if !self.contexts.contains_key(name) {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Context '{}' not found", name),
            )));
        }

        self.contexts.remove(name);

        // If we removed current context, switch to default
        if self.last_context == name {
            self.last_context = "default".to_string();
        }

        self.save()?;
        Ok(())
    }
}

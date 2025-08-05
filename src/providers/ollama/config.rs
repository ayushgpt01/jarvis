use super::types::OllamaModelOptions;
use crate::{AppError, AppResult, model::ModelConfig};

#[derive(Debug, Clone)]
pub struct OllamaConfig {
    pub host: String,
    pub port: u16,
    pub model: String,
    pub options: OllamaModelOptions,
    pub raw: bool,
    pub template: Option<String>,
}

impl OllamaConfig {
    pub fn new(model: String) -> Self {
        Self {
            host: "http://localhost".to_string(),
            port: 11434,
            model,
            options: OllamaModelOptions::default(),
            raw: false,
            template: None,
        }
    }

    pub fn builder() -> OllamaConfigBuilder {
        OllamaConfigBuilder::new()
    }

    pub fn endpoint_url(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

impl ModelConfig for OllamaConfig {
    fn model_name(&self) -> &str {
        &self.model
    }

    fn validate(&self) -> AppResult<()> {
        if self.model.is_empty() {
            return Err(AppError::from("Model name cannot be empty"));
        }
        if self.port == 0 {
            return Err(AppError::from("Port cannot be 0"));
        }
        Ok(())
    }
}

// Builder for OllamaConfig
#[derive(Debug)]
pub struct OllamaConfigBuilder {
    host: String,
    port: u16,
    model: Option<String>,
    options: OllamaModelOptions,
    raw: bool,
    template: Option<String>,
}

impl OllamaConfigBuilder {
    pub fn new() -> Self {
        Self {
            host: "http://localhost".to_string(),
            port: 11434,
            model: None,
            options: OllamaModelOptions::default(),
            raw: false,
            template: None,
        }
    }

    pub fn host(mut self, host: String) -> Self {
        self.host = host;
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn model(mut self, model: String) -> Self {
        self.model = Some(model);
        self
    }

    pub fn options(mut self, options: OllamaModelOptions) -> Self {
        self.options = options;
        self
    }

    pub fn temperature(mut self, temp: f32) -> Self {
        self.options.temperature = Some(temp);
        self
    }

    pub fn raw(mut self, raw: bool) -> Self {
        self.raw = raw;
        self
    }

    pub fn build(self) -> AppResult<OllamaConfig> {
        let model = self
            .model
            .ok_or_else(|| AppError::from("Model is required"))?;

        Ok(OllamaConfig {
            host: self.host,
            port: self.port,
            model,
            options: self.options,
            raw: self.raw,
            template: self.template,
        })
    }
}

impl Default for OllamaConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

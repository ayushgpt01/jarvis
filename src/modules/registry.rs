use super::{Arithmetic, Module, Tool, ToolCallFunction};
use crate::{AppError, AppResult};
use std::collections::HashMap;

pub struct ModuleRegistry {
    modules: HashMap<String, Box<dyn Module>>,
}

impl ModuleRegistry {
    pub fn new() -> ModuleRegistry {
        let mut registry: HashMap<String, Box<dyn Module>> = HashMap::new();
        registry.insert(Arithmetic::name().to_string(), Box::new(Arithmetic::new()));

        ModuleRegistry { modules: registry }
    }

    pub fn list_modules(&self) -> Vec<String> {
        self.modules.keys().cloned().collect()
    }

    pub fn all_tools(&self) -> Vec<Tool> {
        self.modules.values().flat_map(|m| m.tools()).collect()
    }

    pub fn execute(&self, name: &str, func: &ToolCallFunction) -> AppResult<serde_json::Value> {
        let module = self
            .get_module(name)
            .ok_or_else(|| AppError::from(&format!("Module {} not found", name)))?;

        let result = module.run(func)?;
        Ok(result)
    }

    pub fn get_system_prompt(&self) -> String {
        let modules: String = self
            .modules
            .iter()
            .map(|v| format!("- {}: {}\n", v.1.name(), v.1.description()))
            .collect();

        format!("<modules>\n{}</modules>", modules)
    }

    // pub fn register_module(&mut self, name: String, module: Box<dyn Module>) {
    //     self.modules.insert(name, module);
    // }

    fn get_module(&self, name: &str) -> Option<&Box<dyn Module>> {
        self.modules.get(name)
    }
}

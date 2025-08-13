use super::{Math, Module, Tool, ToolCallFunction};
use crate::{AppError, AppResult};
use std::{collections::HashMap, fmt};

pub struct ModuleRegistry {
    modules: HashMap<String, Box<dyn Module + Send + Sync>>,
}

#[allow(dead_code)]
impl ModuleRegistry {
    pub fn new() -> ModuleRegistry {
        let mut registry: HashMap<String, Box<dyn Module + Send + Sync>> = HashMap::new();
        registry.insert(Math::name().to_string(), Box::new(Math::new()));

        ModuleRegistry { modules: registry }
    }

    pub fn empty_registry() -> ModuleRegistry {
        ModuleRegistry {
            modules: HashMap::new(),
        }
    }

    pub fn list_modules(&self) -> Vec<String> {
        self.modules
            .iter()
            .map(|v| format!("{} : {}", v.1.name(), v.1.description()))
            .collect()
    }

    pub fn all_tools(&self) -> Vec<Tool> {
        self.modules.values().flat_map(|m| m.tools()).collect()
    }

    pub fn execute(&self, func: &ToolCallFunction) -> AppResult<serde_json::Value> {
        let module = self
            .get_module(func.module.as_str())
            .ok_or_else(|| AppError::from(&format!("Module {} not found", func.module)))?;

        let result = module.run(func)?;
        Ok(result)
    }

    pub fn get_system_prompt(&self) -> String {
        let modules: String = self
            .modules
            .iter()
            .map(|v| format!("{}\n", v.1.get_prompt()))
            .collect();

        modules
    }

    pub fn register_module(&mut self, name: String, module: Box<dyn Module>) {
        self.modules.insert(name, module);
    }

    pub fn get_module(&self, name: &str) -> Option<&Box<dyn Module + Send + Sync>> {
        self.modules.get(name)
    }
}

// Can add verbose debugging later
impl fmt::Debug for ModuleRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Just show module names instead of their entire contents
        f.debug_struct("ModuleRegistry")
            .field("modules", &self.list_modules())
            .finish()
    }
}

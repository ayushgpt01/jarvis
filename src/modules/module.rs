use serde::{Deserialize, Serialize};

/// Tool definition
#[derive(Debug, Serialize, Clone)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolFunction,
}

#[derive(Debug, Serialize, Clone)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    /// Name of this module
    pub module: String,
    /// Parameters for the tool
    /// serde_json::json!({
    ///     "type": "object",
    ///     "properties": {
    ///     "location": {
    ///         "type": "string",
    ///         "description":
    ///         "The city and state, e.g. San Francisco, CA"  
    ///     },
    ///     "unit": {
    ///         "type": "string",   
    ///         "enum": ["celsius", "fahrenheit"]    
    ///     }
    ///     },
    ///     "required": ["location"]
    /// }),
    pub parameters: serde_json::Value,
}

/// Tool call response
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCall {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolCallFunction,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCallFunction {
    pub name: String,
    /// Name of the module. This is returned by LLM
    pub module: String,
    pub arguments: serde_json::Value,
}

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ModuleError {
    #[error("Unknown function: {0}")]
    UnknownFunction(String),

    #[error("Invalid function input: {0}")]
    InvalidFunctionInput(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),
}

pub type ModuleResult<T, E = ModuleError> = std::result::Result<T, E>;

pub trait Module: Send + Sync {
    /// Name of the current module. This must be unique
    fn name(&self) -> &'static str;
    /// Description is what user will see in the help of cli
    fn description(&self) -> &'static str;
    /// Run method is used to invoke the modules
    fn run(&self, func: &ToolCallFunction) -> ModuleResult<serde_json::Value>;
    /// Available tools in this module in the OpenAI format
    fn tools(&self) -> Vec<Tool>;
}

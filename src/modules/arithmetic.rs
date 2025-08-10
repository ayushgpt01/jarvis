use super::{Module, ModuleError, ModuleResult, ToolCallFunction};
use evalexpr::{Value, eval};
use serde_json::json;

fn value_to_json(val: Value) -> serde_json::Value {
    match val {
        Value::Int(i) => json!(i),
        Value::Float(f) => json!(f),
        Value::String(s) => json!(s),
        Value::Boolean(b) => json!(b),
        Value::Tuple(t) => serde_json::Value::Array(t.into_iter().map(value_to_json).collect()),
        Value::Empty => serde_json::Value::Null,
    }
}

pub struct Arithmetic;

impl Arithmetic {
    pub fn new() -> Arithmetic {
        Arithmetic {}
    }

    pub fn name() -> &'static str {
        "arithmetic"
    }
}

impl Module for Arithmetic {
    fn name(&self) -> &'static str {
        Arithmetic::name()
    }

    fn description(&self) -> &'static str {
        "Allows you to perform arithmetic operations."
    }

    fn run(&self, func: &ToolCallFunction) -> ModuleResult<serde_json::Value> {
        if func.name != "eval" {
            return Err(ModuleError::UnknownFunction(func.name.clone()));
        }

        let expression = func
            .arguments
            .get("expression")
            .ok_or(ModuleError::InvalidFunctionInput(
                "Missing 'expression' argument".into(),
            ))?
            .as_str()
            .ok_or(ModuleError::InvalidFunctionInput(
                "Expected string expression".into(),
            ))?;

        let result = eval(expression)
            .map_err(|e| ModuleError::ExecutionError(format!("Math error: {}", e)))?;

        let json_result = value_to_json(result);
        Ok(json_result)
    }

    fn tools(&self) -> Vec<super::Tool> {
        vec![super::Tool {
            tool_type: "function".to_string(),
            function: super::ToolFunction {
                name: "eval".to_string(),
                description: "Evaluate an expression".to_string(),
                parameters: serde_json::json!({
                  "type": "object",
                  "properties": {
                    "expression": {
                        "type": "string",
                        "description": "Mathematical expression to evaluate (e.g., '2 + 2', 'sqrt(16)')"
                    }
                  },
                  "required": ["expression"]
                }),
            },
        }]
    }
}

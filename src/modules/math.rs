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

pub struct Math;

impl Math {
    pub fn new() -> Math {
        Math {}
    }

    pub fn name() -> &'static str {
        "math"
    }

    fn eval(&self, expression: &str) -> ModuleResult<serde_json::Value> {
        let result = eval(&expression)
            .map_err(|e| ModuleError::ExecutionError(format!("Math error: {}", e)))?;

        let json_result = value_to_json(result);
        Ok(json_result)
    }

    fn pow(&self, base: f64, exponent: f64) -> ModuleResult<serde_json::Value> {
        let result = base.powf(exponent);
        Ok(json!(result))
    }

    fn sqrt(&self, value: f64) -> ModuleResult<serde_json::Value> {
        let result = value.sqrt();
        Ok(json!(result))
    }

    fn parse_float(&self, value_json: &serde_json::Value) -> ModuleResult<f64> {
        match value_json {
            serde_json::Value::Number(n) => n.as_f64().ok_or_else(|| {
                ModuleError::InvalidFunctionInput("Value is not a valid number".into())
            }),
            serde_json::Value::String(s) => s.parse().map_err(|_| {
                ModuleError::InvalidFunctionInput("Value string is not a valid number".into())
            }),
            _ => Err(ModuleError::InvalidFunctionInput(
                "Expected a number or a string for 'value'".into(),
            )),
        }
    }
}

impl Module for Math {
    fn name(&self) -> &'static str {
        Math::name()
    }

    fn description(&self) -> &'static str {
        "Allows you to perform mathematical operations, including arithmetic, exponents, and square roots."
    }

    fn get_prompt(&self) -> &'static str {
        r#"
- **math**: Allows you to perform mathematical operations, including arithmetic, exponents, and square roots.
  - **Rules**:
    - For all arithmetic expressions, you MUST include a floating-point number in the tool call (e.g., `2.0 * 5` instead of `2 * 5`) to ensure accurate results.
    - Use the `eval` function for general arithmetic expressions (e.g., `(2.0 * 5) - 10`).
    - Use the `pow` function for exponents (e.g., `pow(5, 3)`).
    - Use the `sqrt` function for square roots (e.g., `sqrt(81)`)."#
    }

    fn run(&self, func: &ToolCallFunction) -> ModuleResult<serde_json::Value> {
        // In `run` function
        match func.name.as_str() {
            "eval" => {
                let expression = func
                    .arguments
                    .get("expression")
                    .ok_or_else(|| {
                        ModuleError::InvalidFunctionInput("Missing 'expression' argument".into())
                    })?
                    .as_str()
                    .ok_or_else(|| {
                        ModuleError::InvalidFunctionInput("Expected string expression".into())
                    })?;

                self.eval(expression)
            }
            "pow" => {
                let base_json = func
                    .arguments
                    .get("base")
                    .ok_or_else(|| {
                        ModuleError::InvalidFunctionInput("Missing 'base' argument".into())
                    })?
                    .clone();

                let exponent_json = func
                    .arguments
                    .get("exponent")
                    .ok_or_else(|| {
                        ModuleError::InvalidFunctionInput("Missing 'exponent' argument".into())
                    })?
                    .clone();

                let base: f64 = self.parse_float(&base_json)?;
                let exponent: f64 = self.parse_float(&exponent_json)?;

                self.pow(base, exponent)
            }
            "sqrt" => {
                let value_json = func
                    .arguments
                    .get("value")
                    .ok_or_else(|| {
                        ModuleError::InvalidFunctionInput("Missing 'value' argument".into())
                    })?
                    .clone();

                let value: f64 = self.parse_float(&value_json)?;

                self.sqrt(value)
            }
            _ => Err(ModuleError::UnknownFunction(func.name.clone())),
        }
    }

    fn tools(&self) -> Vec<super::Tool> {
        vec![
            // Eval function
            super::Tool {
                tool_type: "function".to_string(),
                function: super::ToolFunction {
                    name: "eval".to_string(),
                    module: Self::name().to_string(),
                    description: "Evaluate a mathematical expression".to_string(),
                    parameters: serde_json::json!({
                      "type": "object",
                      "properties": {
                        "expression": {
                            "type": "string",
                            "description": "Mathematical expression to evaluate (e.g., '2.0 + 2.0', '5^3')"
                        }
                      },
                      "required": ["expression"]
                    }),
                },
            },
            // Pow function
            super::Tool {
                tool_type: "function".to_string(),
                function: super::ToolFunction {
                    name: "pow".to_string(),
                    module: Self::name().to_string(),
                    description: "Raises a base to the power of an exponent".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "base": { "type": "number", "description": "The base number" },
                            "exponent": { "type": "number", "description": "The exponent" }
                        },
                        "required": ["base", "exponent"]
                    }),
                },
            },
            // Sqrt function
            super::Tool {
                tool_type: "function".to_string(),
                function: super::ToolFunction {
                    name: "sqrt".to_string(),
                    module: Self::name().to_string(),
                    description: "Calculates the square root of a number".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "value": { "type": "number", "description": "The number to find the square root of" }
                        },
                        "required": ["value"]
                    }),
                },
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval() {
        let math = Math::new();
        let result = math
            .run(&ToolCallFunction {
                name: "eval".to_string(),
                module: Math::name().to_string(),
                arguments: json!({ "expression": "2000.0 * 2122.0" }),
            })
            .unwrap();
        assert_eq!(result, json!(4244000.0));
    }

    #[test]
    fn test_eval_with_division() {
        let math = Math::new();
        let result = math
            .run(&ToolCallFunction {
                name: "eval".to_string(),
                module: Math::name().to_string(),
                arguments: json!({ "expression": "(2000.0 * 2122.0) / (22124.0 * 900.0)" }),
            })
            .unwrap();
        let expected_float = (2000.0 * 2122.0) / (22124.0 * 900.0);

        let actual_float: f64 = serde_json::from_value(result).unwrap();

        println!("Expected: {}, Actual: {}", expected_float, actual_float);
        let epsilon = 1e-9;
        assert!((actual_float - expected_float).abs() < epsilon);
    }

    #[test]
    fn test_pow_number() {
        let math = Math::new();
        let result = math
            .run(&ToolCallFunction {
                name: "pow".to_string(),
                module: Math::name().to_string(),
                arguments: json!({ "base": 5, "exponent": 3 }),
            })
            .unwrap();
        assert_eq!(result, json!(125.0));
    }

    #[test]
    fn test_pow_string() {
        let math = Math::new();
        let result = math
            .run(&ToolCallFunction {
                name: "pow".to_string(),
                module: Math::name().to_string(),
                arguments: json!({ "base": "5", "exponent": "3" }),
            })
            .unwrap();
        assert_eq!(result, json!(125.0));
    }

    #[test]
    fn test_sqrt_number() {
        let math = Math::new();
        let result = math
            .run(&ToolCallFunction {
                name: "sqrt".to_string(),
                module: Math::name().to_string(),
                arguments: json!({ "value": 81 }),
            })
            .unwrap();
        assert_eq!(result, json!(9.0));
    }

    #[test]
    fn test_sqrt_string() {
        let math = Math::new();
        let result = math
            .run(&ToolCallFunction {
                name: "sqrt".to_string(),
                module: Math::name().to_string(),
                arguments: json!({ "value": "81" }),
            })
            .unwrap();
        assert_eq!(result, json!(9.0));
    }
}

use crate::{
    AppError, AppResult, Cli,
    modules::ModuleRegistry,
    providers::{OllamaConfig, create_ollama_client},
    streaming::{OutputStreamer, StreamEvent, create_cli_streamer},
    utils::get_file_content,
};
use std::sync::Arc;

const SYSTEM_PROMPT: &str = r#"You are a helpful assistant. You are given a task and you must answer the user's query following the rules and using available modules when appropriate.

<context>
__CONTEXT__
</context>

<modules>
__MODULES__
</modules>

<rules>
- If you don't know, say so.
- If you are not sure, ask for clarification.
- Answer in the same language as the user query.
- Answer directly and without using XML tags.
- When proposing files, use the file block syntax (see examples below).
- For Markdown files, use four backticks to wrap the file content so inner code blocks are preserved.

--- TOOL / MODULE USAGE (IMPORTANT) ---
You may call one or more tools (functions) that belong to modules. Each tool in the provided tool registry contains:
- type: "function"
- function: { name, module, parameters }

When you decide to call tools you MUST follow these rules exactly:

1. **Only** return a single JSON _array_ containing the tool call objects. Do not include any additional text outside the JSON array. The client expects pure JSON when you invoke tools.
2. Each tool call object must match this format exactly:

```json
[
  {
    "type": "function",
    "function": {
      "name": "<tool_name>",
      "module": "<module_name>",
      "arguments": { /* arguments matching the tool's parameters schema */ }
    }
  }
]
````

3. The `module` field is required and must match the module name shown in the `<modules>` section exactly.
4. Argument objects must strictly conform to the parameter schema provided with the tool (no missing required fields and no extra unexpected fields).
5. You may return multiple tool call objects in the array (for multi-step operations).
6. If no tool is required, respond normally as plain text (not JSON).

Examples:

* If the user asks: "Compute 2+2", and the `arithmetic.eval` tool is appropriate, return:

```json
[
  {
    "type": "function",
    "function": {
      "name": "eval",
      "module": "arithmetic",
      "arguments": { "expression": "2+2" }
    }
  }
]
```

* If no tool is needed, reply with plain language and not JSON.

\--- FILE BLOCK EXAMPLES ---
If you propose a file, represent it as a code block with a `name` in the header. Example TypeScript file:

```typescript name=filename.ts
// file contents here
```

Markdown file example (use four backticks to wrap the file):

````markdown name=filename.md
```js
console.log("inner code block preserved");
```
````

</rules>"#;

const HOST: &str = "http://localhost";
const PORT: u16 = 11434;
const LLM_MODEL: &str = "llama3.2";

pub async fn process_prompt(cli: &Cli, module_registry: &Arc<ModuleRegistry>) -> AppResult<()> {
    let mut streamer = create_cli_streamer(false);

    // Determine which tools to expose based on --module
    let (tools_for_payload, modules_for_prompt) = if let Some(module_name) = cli.module.as_deref() {
        let module = module_registry
            .get_module(module_name)
            .ok_or_else(|| AppError::from(&format!("Module {} not found", module_name)))?;

        // Machine-readable for LLM
        let tool_schemas = module.tools();

        // Human-readable for system prompt
        let module_desc = format!("- {}: {}\n", module.name(), module.description());

        (tool_schemas, module_desc)
    } else {
        // All tools (machine-readable)
        let tool_schemas = module_registry.all_tools();

        // All modules description (human-readable)
        let module_desc = module_registry.get_system_prompt();

        (tool_schemas, module_desc)
    };

    let config = OllamaConfig::new()
        .host(HOST.to_string())
        .model(LLM_MODEL.to_string())
        .port(PORT)
        .tools(tools_for_payload)
        // .options(options)
        .build()?;

    let mut client = create_ollama_client(config, module_registry.clone())?;

    let prompt_text: Option<String> = cli.text()?;
    let prompt = prompt_text.as_deref().ok_or(AppError::InvalidInput)?;

    let file_path = cli.input.as_deref().unwrap_or("");

    // Load file content if provided
    let file_content = if file_path.is_empty() {
        String::new()
    } else {
        streamer
            .handle_event(StreamEvent::Status(format!("Loading file: {}", file_path)))
            .await?;

        log::info!("Loading file content from: {}", file_path);
        get_file_content(file_path.to_string())?
    };

    let oneshot_prompt = SYSTEM_PROMPT
        .replace("__CONTEXT__", &file_content)
        .replace("__MODULES__", &modules_for_prompt);

    log::info!("One shot prompt: {}", oneshot_prompt);

    client.set_system_message(&oneshot_prompt);
    client.chat_streaming(prompt, &mut streamer).await?;

    streamer.finish().await?;
    log::info!("Prompt processing completed successfully");

    Ok(())
}

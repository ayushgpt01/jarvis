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
- When a tool is called, you MUST ONLY return the JSON array. Do not include any other text, reasoning, or explanations outside the JSON array.
- If no tool is required, respond normally as plain text (not JSON).
- When a tool returns a result, you will see it in the conversation history as 'TOOL_RESULT: ...'. Your final answer MUST be a direct, concise summary of the result in natural language. Do not add any conversational text, explanations, or extraneous details unless asked.
- When a numerical result is obtained, present it as a plain number without any additional text, symbols, or currency signs.
- You must only use the function names provided in the module registry. Do not invent new functions.

--- TOOL / MODULE USAGE ---
You have access to a set of tools (functions) within modules. Each tool has a `name`, `module`, and `parameters`. To use a tool, you must return a **single JSON array** of one or more tool call objects.

**IMPORTANT RULES for Tool Calls:**
1. You **MUST** return only a JSON array. No text, explanations, or other characters should appear outside of the JSON.
2. Each object in the array **MUST** strictly follow this structure:

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

3. The `module` field is required and must match the module name shown in the `<modules>` section exactly.
4. Argument objects must strictly conform to the parameter schema provided with the tool (no missing required fields and no extra unexpected fields).
5. You may return multiple tool call objects in the array (for multi-step operations).

Examples:

* If the user asks: "What is 10 divided by 3?", and the `math.eval` tool is appropriate, return:

[
  {
    "type": "function",
    "function": {
      "name": "eval",
      "module": "math",
      "arguments": { "expression": "10.0/3" }
    }
  }
]

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
        let module_desc = format!("{}\n", module.get_prompt());

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

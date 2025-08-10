use crate::{
    AppError, AppResult, Cli, modules, providers,
    streaming::{self, OutputStreamer, StreamEvent},
    utils::get_file_content,
};

const SYSTEM_PROMPT: &str = r#"You are a helpful assistant. You are given a task and you need to answer the query based on the rules. (user query, context, rules, modules in xml format). Modules contain the functions that you can use to answer the query.
<context>
__CONTEXT__
</context>

<rules>
- If you don't know, just say so.
- If you are not sure, ask for clarification.
- Answer in the same language as the user query.
- Answer directly and without using xml tags.
- Whenever proposing a file use the file block syntax.
- Files must be represented as code blocks with their `name` in the header.
Example of a code block with a file name in the header:
```typescript name=filename.ts
contents of file
```

- For Markdown files, you must use four opening and closing backticks (````) to ensure that code blocks inside are escaped.
Example of a code block for a Markdown file:
````markdown name=filename.md
```code block inside file```
````
</rules>
"#;

const HOST: &str = "http://localhost";
const PORT: u16 = 11434;
const LLM_MODEL: &str = "llama3.2";

pub async fn process_prompt(cli: &Cli, module_registry: &modules::ModuleRegistry) -> AppResult<()> {
    let mut streamer = streaming::create_cli_streamer(false);
    let config = providers::OllamaConfig::new()
        .host(HOST.to_string())
        .model(LLM_MODEL.to_string())
        .port(PORT)
        .tools(module_registry.all_tools())
        // .options(options)
        .build()?;

    let mut client = providers::create_ollama_client(config)?;

    let prompt_text: Option<String> = cli.text()?;
    let prompt = prompt_text.as_deref().ok_or(AppError::InvalidInput)?;
    let module_ref = cli.module.as_deref().unwrap_or("");
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

    let oneshot_prompt = SYSTEM_PROMPT.replace("__CONTEXT__", &file_content);

    client.set_system_message(&oneshot_prompt);
    client.chat_streaming(prompt, &mut streamer).await?;

    streamer.finish().await?;
    log::info!("Prompt processing completed successfully");

    Ok(())
}

use crate::{
    cli::Cli,
    core::ollama::OllamaGenerate,
    error::AppError,
    streaming::streamer::{OutputStreamer, StreamEvent},
    utils::functions::get_file_content,
};

const SYSTEM_PROMPT: &str = r#"You are a helpful assistant. You are given a task and you need to answer the query based on the rules. (user query, context, rules, modules in xml format). Modules contain the functions that you can use to answer the query.

<modules>
__MODULES__
</modules>

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

<user_query>
__USER_QUERY__
</user_query>
"#;

const HOST: &str = "http://localhost";
const PORT: u16 = 11434;
const LLM_MODEL: &str = "llama3.2";

pub async fn process_prompt(cli: &Cli, streamer: &mut impl OutputStreamer) -> Result<(), AppError> {
    streamer
        .handle_event(StreamEvent::Status("Initializing...".to_string()))
        .await?;

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

    log::info!("Processing prompt with {} characters", prompt.len());

    let oneshot_prompt = SYSTEM_PROMPT
        .replace("__USER_QUERY__", prompt)
        .replace("__MODULES__", module_ref)
        .replace("__CONTEXT__", &file_content);

    log::debug!("Final Prompt: {}", oneshot_prompt);

    let ollama_generate_config = OllamaGenerate {
        host: HOST.to_string(),
        port: PORT,
        model: LLM_MODEL.to_string(),
        stream: Some(true),
        raw: Some(false),
        options: None,
        template: None,
        system: None,
    };

    ollama_generate_config
        .send_prompt_streaming(&oneshot_prompt, streamer)
        .await?;

    streamer.finish().await?;
    log::info!("Prompt processing completed successfully");

    Ok(())
}

# xNet Runtime Integration Guide

## Purpose
This module provides real AI inference capabilities by integrating with Ollama.

## Architecture
- **Ollama API Client**: HTTP client to `localhost:11434`
- **Model Manager**: List, pull, and manage models
- **Inference Engine**: Execute real text generation tasks

## API Reference
### `generate(model: String, prompt: String) -> Result<String>`
Sends a prompt to Ollama and returns the generated response.

### `list_models() -> Result<Vec<String>>`
Lists all locally available models.

### `pull_model(name: String) -> Result<()>`
Downloads a new model from Ollama registry.

## Example Usage
```rust
use xnet_runtime::OllamaRuntime;

let runtime = OllamaRuntime::new("http://localhost:11434");
let response = runtime.generate("tinyllama", "Why is the sky blue?").await?;
println!("Response: {}", response);
```

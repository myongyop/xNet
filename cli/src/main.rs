use anyhow::Result;
use tokio::io::{self, AsyncBufReadExt};
use xnet_core::{InferenceTask, NetworkInterface};
use xnet_network::P2PNode;
use xnet_runtime::OllamaRuntime;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting xNet Agent (PoC)...");
    
    // 1. Start P2P Node
    let node = P2PNode::new().await?;
    println!("P2P Node started!");

    // 2. Start Runtime (Lazy init, just checking connection or defining it)
    let _runtime = OllamaRuntime::new("http://localhost:11434");
    println!("Runtime initialized (connected to Ollama at localhost:11434)");

    println!("commands: [publish <prompt>, exit]");

    // 3. Command Loop
    let stdin = io::stdin();
    let mut reader = io::BufReader::new(stdin).lines();

    while let Some(line) = reader.next_line().await? {
        let line = line.trim();
        if line == "exit" {
            break;
        } else if line.starts_with("publish ") {
            let prompt = line.trim_start_matches("publish ").to_string();
            let task = InferenceTask::new("task-id-1", "llama3", prompt);
            node.publish_task(task).await.map_err(|e| anyhow::anyhow!(e))?;
            println!("Task published!");
        }
    }

    Ok(())
}

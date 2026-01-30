use tauri::{Emitter, Manager};
use xnet_network::{P2PNode, NetworkEvent, NetworkInterface};
use tokio::sync::Mutex;
use std::sync::Arc;

struct AppState {
    node: Arc<Mutex<Option<P2PNode>>>,
}

#[derive(serde::Serialize)]
struct SystemSpecs {
    cpu_cores: usize,
    total_memory: u64,
    role_recommendation: String,
}

#[tauri::command]
fn get_system_specs() -> SystemSpecs {
    use sysinfo::System;
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu_cores = sys.cpus().len();
    let total_memory = sys.total_memory(); // Bytes
    
    // Simple heuristic: > 16GB RAM + > 8 Cores = Likely High End (Muscle)
    // In reality, we need actual GPU detection (e.g. via nvidia-smi command check)
    let is_high_end = total_memory > 16 * 1024 * 1024 * 1024 && cpu_cores >= 8;
    
    let role_recommendation = if is_high_end {
        "Muscle".to_string()
    } else {
        "Nerve".to_string()
    };

    SystemSpecs {
        cpu_cores,
        total_memory,
        role_recommendation,
    }
}

#[tauri::command]
async fn start_node(state: tauri::State<'_, AppState>, app: tauri::AppHandle, bootnode: Option<String>) -> Result<String, String> {
    let mut node_guard = state.node.lock().await;

    if node_guard.is_some() {
        return Ok("Node already running".to_string());
    }

    let mut bootnodes = Vec::new();
    if let Some(addr_str) = bootnode {
        if !addr_str.is_empty() {
             use std::str::FromStr;
             if let Ok(addr) = libp2p::Multiaddr::from_str(&addr_str) {
                 bootnodes.push(addr);
             } else {
                 return Err("Invalid bootnode address".to_string());
             }
        }
    }

    // Identity Persistence
    let app_handle = app.clone();
    let data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    }
    let keypath = data_dir.join("identity.bin");
    
    let keypair_bytes = if keypath.exists() {
        std::fs::read(&keypath).map_err(|e| e.to_string())?
    } else {
        use libp2p::identity;
        let keys = identity::Keypair::generate_ed25519();
        let bytes = keys.to_protobuf_encoding().map_err(|e| e.to_string())?;
        std::fs::write(&keypath, &bytes).map_err(|e| e.to_string())?;
        bytes
    };

    // Wallet Persistence
    let wallet_path = data_dir.join("wallet.json");
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Wallet {
        balance: f64,
    }

    let initial_balance = if wallet_path.exists() {
        let data = std::fs::read_to_string(&wallet_path).map_err(|e| e.to_string())?;
        serde_json::from_str::<Wallet>(&data).map_err(|e| e.to_string())?.balance
    } else {
        0.0
    };

    let node = P2PNode::new(bootnodes, Some(keypair_bytes), initial_balance).await.map_err(|e| e.to_string())?;
    *node_guard = Some(node.clone());

    let mut rx = node.subscribe();
    
    // Spawn event listener
    let app_event = app.clone();
    let wallet_path_clone = wallet_path.clone();

    tauri::async_runtime::spawn(async move {
        while let Ok(event) = rx.recv().await {
            match event {
                NetworkEvent::PeerConnected(id) => {
                    let _ = app_event.emit("peer-connected", id);
                }
                NetworkEvent::PeerDisconnected(id) => {
                    let _ = app_event.emit("peer-disconnected", id);
                }
                NetworkEvent::Message(msg) => {
                    let _ = app_event.emit("log-message", msg);
                }
                NetworkEvent::TaskReceived(task) => {
                    let _ = app_event.emit("task-received", task);
                }
                NetworkEvent::DhtEvent(msg) => {
                    let _ = app_event.emit("dht-event", msg);
                }
                NetworkEvent::MetricsUpdated(metrics) => {
                    // Save wallet balance
                    let wallet = Wallet { balance: metrics.credits };
                    if let Ok(json) = serde_json::to_string(&wallet) {
                        let _ = std::fs::write(&wallet_path_clone, json);
                    }
                    let _ = app_event.emit("metrics-updated", metrics);
                }
                NetworkEvent::PipelineEvent(event) => {
                    let _ = app_event.emit("pipeline-event", event);
                }
                NetworkEvent::VerificationEvent(event) => {
                    let _ = app_event.emit("verification-event", event);
                }
                NetworkEvent::FLEvent(event) => {
                    let _ = app_event.emit("fl-event", event);
                }
            }
        }
    });

    // Spawn Idle Detection (Simple Polling)
    let app_idle = app.clone();
    let node_idle = node.clone();
    
    tauri::async_runtime::spawn(async move {
        use user_idle::UserIdle;
        use std::time::Duration;
        
        let mut is_muscle_mode = false;
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            
            // Check idle time
            let idle_seconds = UserIdle::get_time().map(|i| i.as_seconds()).unwrap_or(0);
            
            if idle_seconds > 60 && !is_muscle_mode {
                is_muscle_mode = true;
                let _ = app_idle.emit("mode-change", "Muscle (Idle)");
                
                // Announce Provider Capability
                if let Err(e) = node_idle.announce_provider().await {
                   println!("Failed to announce provider: {:?}", e);
                }
                
            } else if idle_seconds < 5 && is_muscle_mode {
                is_muscle_mode = false;
                let _ = app_idle.emit("mode-change", "Nerve (Active)");
            }
        }
    });

    // Start HTTP API Server
    let api_node = node.clone();
    tauri::async_runtime::spawn(async move {
        use warp::Filter;
        
        // POST /api/v1/task
        let task_route = warp::post()
            .and(warp::path("api"))
            .and(warp::path("v1"))
            .and(warp::path("task"))
            .and(warp::body::json())
            .then(move |req: CreateTaskRequest| {
                async move {
                    let task_id = uuid::Uuid::new_v4().to_string();
                    println!("API received task: {} - {}", task_id, req.prompt);
                    
                    // Process locally (single node mode)
                    use xnet_core::RuntimeInterface;
                    let runtime = xnet_runtime::OllamaRuntime::new("http://localhost:11434");
                    
                    match runtime.generate(&req.model, &req.prompt).await {
                        Ok(response) => {
                            let preview: String = response.chars().take(50).collect();
                            println!("[LOCAL AI] Task {} completed: {}...", task_id, preview);
                            let escaped = response.replace("\"", "\\\"").replace("\n", "\\n");
                            warp::reply::with_status(
                                format!("{{\"status\": \"completed\", \"task_id\": \"{}\", \"result\": \"{}\"}}", task_id, escaped),
                                warp::http::StatusCode::OK,
                            )
                        },
                        Err(e) => {
                            println!("[LOCAL AI] Task {} failed: {}", task_id, e);
                            warp::reply::with_status(
                                format!("{{\"error\": \"{}\"}}", e),
                                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                            )
                        }
                    }
                }
            });
        
        // Fix for "AsRef not general enough" - ensure filter is boxed
        let task_route = task_route.boxed();

        println!("Starting API Server on 0.0.0.0:3030");
        warp::serve(task_route).run(([0, 0, 0, 0], 3030)).await;
    });

    Ok("Node started successfully".to_string())
}

#[derive(serde::Deserialize)]
struct CreateTaskRequest {
    model: String,
    prompt: String,
}

#[tauri::command]
async fn test_pipeline_event(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let node_guard = state.node.lock().await;
    if let Some(node) = node_guard.as_ref() {
        let event = xnet_core::PipelineEvent::InitSession {
            session_id: "test-session-123".to_string(),
            model: "llama-3-8b".to_string(),
        };
        node.publish_pipeline_event(event).await.map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Node not running".to_string())
    }
}

#[tauri::command]
async fn test_verification_event(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let node_guard = state.node.lock().await;
    if let Some(node) = node_guard.as_ref() {
        let event = xnet_core::VerificationEvent::ChallengeIssued(xnet_core::Challenge {
            target_session_id: "test-session-123".to_string(),
            target_layer: 10,
            challenger_id: "node-challenger-xyz".to_string(),
        });
        node.publish_verification_event(event).await.map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Node not running".to_string())
    }
}

#[tauri::command]
async fn list_models() -> Result<Vec<String>, String> {
    use xnet_runtime::OllamaRuntime;
    let runtime = OllamaRuntime::new("http://localhost:11434");
    runtime.list_models().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn test_fl_event(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let node_guard = state.node.lock().await;
    if let Some(node) = node_guard.as_ref() {
        let event = xnet_core::FLEvent::LocalUpdate(xnet_core::FLUpdate {
            task_id: "fl-task-mnist-01".to_string(),
            node_id: "node-worker-abc".to_string(),
            round: 1,
            gradients: vec![0.01; 10],
            metrics: "loss: 0.042".to_string(),
        });
        node.publish_fl_event(event).await.map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Node not running".to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState { node: Arc::new(Mutex::new(None)) })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![start_node, get_system_specs, test_pipeline_event, test_verification_event, test_fl_event, list_models])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

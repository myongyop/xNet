mod behaviour;

use anyhow::Result;
use crate::behaviour::{RhizomeBehaviour, RhizomeBehaviourEvent};
use libp2p::{
    futures::StreamExt,
    gossipsub, kad, mdns, noise, tcp, yamux, SwarmBuilder,
    identity,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tokio::sync::{mpsc, broadcast};
use xnet_core::{DynError, InferenceTask, PipelineEvent, VerificationEvent, FLEvent, RuntimeInterface};
use xnet_runtime::OllamaRuntime;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

pub use xnet_core::NetworkInterface;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetworkEvent {
    PeerConnected(String),
    PeerDisconnected(String),
    TaskReceived(InferenceTask),
    Message(String),
    DhtEvent(String),
    MetricsUpdated(NodeMetrics),
    PipelineEvent(PipelineEvent),
    VerificationEvent(VerificationEvent),
    FLEvent(FLEvent),
}

use xnet_core::NodeMetrics;

#[derive(Clone)]
pub struct P2PNode {
    sender: mpsc::Sender<Command>,
    event_sender: broadcast::Sender<NetworkEvent>,
}

enum Command {
    PublishTask(InferenceTask),
    PublishPipeline(PipelineEvent),
    PublishVerification(VerificationEvent),
    PublishFL(FLEvent),
    StartProviding,
}

use libp2p::Multiaddr;

impl P2PNode {
    pub async fn new(bootnodes: Vec<Multiaddr>, keypair_bytes: Option<Vec<u8>>, initial_credits: f64) -> Result<Self> {
        let (sender, mut receiver) = mpsc::channel(32);
        let (event_sender, _) = broadcast::channel(100);
        let event_sender_clone = event_sender.clone();
        
        let command_sender = sender.clone(); // Clone for the event loop

        tokio::spawn(async move {
            // Initialize Ollama Runtime
            let runtime = OllamaRuntime::new("http://localhost:11434");
            
            let id_keys = if let Some(bytes) = keypair_bytes {
                identity::Keypair::from_protobuf_encoding(&bytes)?
            } else {
                identity::Keypair::generate_ed25519()
            };
            let peer_id = id_keys.public().to_peer_id();
            println!("Local Peer ID: {}", peer_id);
            
            // Note: We use command_sender inside the loop for self-messaging if needed
            let sender = command_sender; // Rename for clarity inside, or just use command_sender

            let mut swarm = SwarmBuilder::with_existing_identity(id_keys)
                .with_tokio()
                .with_tcp(
                    tcp::Config::default(),
                    noise::Config::new,
                    yamux::Config::default,
                )?
                .with_quic()
                .with_behaviour(|key| {
                    let peer_id = key.public().to_peer_id();
                    
                    // Gossipsub config
                    let message_id_fn = |message: &gossipsub::Message| {
                        let mut s = DefaultHasher::new();
                        message.data.hash(&mut s);
                        gossipsub::MessageId::from(s.finish().to_string())
                    };
                    let gossipsub_config = gossipsub::ConfigBuilder::default()
                        .heartbeat_interval(Duration::from_secs(10))
                        .validation_mode(gossipsub::ValidationMode::Strict)
                        .message_id_fn(message_id_fn)
                        .max_transmit_size(256 * 1024) // Increase max size for Tensors
                        .build()
                        .map_err(|msg| std::io::Error::new(std::io::ErrorKind::Other, msg))?;

                    let gossipsub = gossipsub::Behaviour::new(
                        gossipsub::MessageAuthenticity::Signed(key.clone()),
                        gossipsub_config,
                    )?;

                    // mDNS config
                    let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), peer_id)?;
                    
                    // Kademlia (DHT) config
                    let store = kad::store::MemoryStore::new(peer_id);
                    let kad_config = kad::Config::default();
                    let kad = kad::Behaviour::with_config(peer_id, store, kad_config);

                    Ok(RhizomeBehaviour { gossipsub, mdns, kad })
                })?
                .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
                .build();

            swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
            swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;

            // Subscribe to topics
            let tasks_topic = gossipsub::IdentTopic::new("xnet/tasks/v1");
            swarm.behaviour_mut().gossipsub.subscribe(&tasks_topic)?;

            let pipeline_topic = gossipsub::IdentTopic::new("xnet/pipeline/v1");
            swarm.behaviour_mut().gossipsub.subscribe(&pipeline_topic)?;

            let verification_topic = gossipsub::IdentTopic::new("xnet/verification/v1");
            swarm.behaviour_mut().gossipsub.subscribe(&verification_topic)?;

            let fl_topic = gossipsub::IdentTopic::new("xnet/fl/v1");
            swarm.behaviour_mut().gossipsub.subscribe(&fl_topic)?;
            
            // Set Kademlia mode to Server
            swarm.behaviour_mut().kad.set_mode(Some(kad::Mode::Server));

            // Bootstrap
            for addr in bootnodes {
                println!("Bootstrapping with {}", addr);
                // Extract PeerId from Multiaddr if present (required for add_address)
                // For Kademlia add_address, we need PeerId.
                // Assuming multiaddr ends with /p2p/<peer_id>
                if let Some(libp2p::multiaddr::Protocol::P2p(peer_id)) = addr.iter().last() {
                     println!("Adding bootnode: {} ({})", peer_id, addr);
                     swarm.behaviour_mut().kad.add_address(&peer_id, addr);
                } else {
                     println!("Invalid bootnode address (missing PeerID): {}", addr);
                }
            }
            if let Err(e) = swarm.behaviour_mut().kad.bootstrap() {
                println!("Bootstrap error: {:?}", e);
            }

            let start_time = std::time::Instant::now();
            let mut metrics = NodeMetrics::new();
            metrics.credits = initial_credits;
            let mut last_metrics_update = std::time::Instant::now();

            loop {
                // Emit metrics every 5 seconds
                if last_metrics_update.elapsed() > Duration::from_secs(5) {
                    metrics.uptime_seconds = start_time.elapsed().as_secs();
                    
                    // Simple Reward Calculation Logic (Proof of Contribution)
                    // 0.1 Credit per minute of uptime
                    // 5.0 Credits per task processed
                    // 1.0 Credit per task relayed
                    let uptime_credits = (metrics.uptime_seconds as f64 / 60.0) * 0.1;
                    let processing_credits = metrics.tasks_processed as f64 * 5.0;
                    let relay_credits = metrics.tasks_relayed as f64 * 1.0;
                    
                    metrics.credits = uptime_credits + processing_credits + relay_credits;

                    let _ = event_sender_clone.send(NetworkEvent::MetricsUpdated(metrics.clone()));
                    last_metrics_update = std::time::Instant::now();
                }

                tokio::select! {
                     event = swarm.select_next_some() => {
                        match event {
                             libp2p::swarm::SwarmEvent::NewListenAddr { address, .. } => {
                                 println!("Listening on {:?}", address);
                             },
                             libp2p::swarm::SwarmEvent::Behaviour(RhizomeBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                                 for (peer_id, multiaddr) in list {
                                     println!("mDNS discovered a new peer: {}", peer_id);
                                     swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                                     swarm.behaviour_mut().kad.add_address(&peer_id, multiaddr); // Add to DHT
                                     let _ = event_sender_clone.send(NetworkEvent::PeerConnected(peer_id.to_string()));
                                 }
                             },
                             libp2p::swarm::SwarmEvent::Behaviour(RhizomeBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                                 for (peer_id, _multiaddr) in list {
                                     println!("mDNS discover peer has expired: {}", peer_id);
                                     swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                                     let _ = event_sender_clone.send(NetworkEvent::PeerDisconnected(peer_id.to_string()));
                                 }
                             },
                             libp2p::swarm::SwarmEvent::Behaviour(RhizomeBehaviourEvent::Gossipsub(gossipsub::Event::Message { propagation_source: peer_id, message_id: _, message })) => {
                                 metrics.tasks_relayed += 1;
                                 let topic = message.topic.as_str();

                                 if topic == "xnet/tasks/v1" {
                                     // Handle Task with REAL Inference
                                     if let Ok(task) = serde_json::from_slice::<InferenceTask>(&message.data) {
                                         metrics.tasks_processed += 1;
                                         println!("Got task from {}: {:?}", peer_id, task.id);
                                         let _ = event_sender_clone.send(NetworkEvent::TaskReceived(task.clone()));
                                         
                                         // Perform real inference using Ollama
                                         let runtime_clone = runtime.clone();
                                         let event_sender_for_result = event_sender_clone.clone();
                                         tokio::spawn(async move {
                                             match runtime_clone.generate(&task.model_name, &task.prompt).await {
                                                 Ok(response) => {
                                                     let preview = if response.len() > 50 {
                                                         format!("{}...", &response[..50])
                                                     } else {
                                                         response.clone()
                                                     };
                                                     println!("[REAL AI] Task {} completed: {}", task.id, preview);
                                                     let msg = format!("[AI Response] {}", response);
                                                     let _ = event_sender_for_result.send(NetworkEvent::Message(msg));
                                                 },
                                                 Err(e) => {
                                                     println!("[REAL AI] Task {} failed: {}", task.id, e);
                                                     let msg = format!("[AI Error] {}", e);
                                                     let _ = event_sender_for_result.send(NetworkEvent::Message(msg));
                                                 }
                                             }
                                         });
                                     }
                                 } else if topic == "xnet/pipeline/v1" {
                                     // Handle Pipeline Event
                                     if let Ok(event) = serde_json::from_slice::<PipelineEvent>(&message.data) {
                                         println!("Got pipeline event from {}: {:?}", peer_id, event);
                                         let _ = event_sender_clone.send(NetworkEvent::PipelineEvent(event.clone()));
                                         
                                         // Simulation Logic for Swarm Parallelism
                                         // In a real system, this would check if *this* node is the next in the schedule.
                                         // Here, we just probabilistically or deterministically advance the state to simulate the flow.
                                         
                                         // To avoid infinite loops in a broadcast network, we need a condition.
                                         // For the demo, let's say the node that *initiated* the test (via UI) is observing.
                                         // We need a mechanism for nodes to react.
                                         // Let's make it simple: If I receive InitSession, I start the chain.
                                         // If I receive ForwardPass, I continue it if I'm "lucky" (simulating role assignment).
                                         
                                         match event {
                                             PipelineEvent::InitSession { session_id, model: _ } => {
                                                 // Start the chain! Layer 0-10
                                                 let next_event = PipelineEvent::ForwardPass {
                                                     session_id,
                                                     layer_start: 0,
                                                     tensor: xnet_core::Tensor { shape: vec![1, 4096], data: vec![0.1; 10] }
                                                 };
                                                 // Broadcast after a delay (simulating compute)
                                                 let swarm_sender = sender.clone();
                                                 tokio::spawn(async move {
                                                     tokio::time::sleep(Duration::from_secs(2)).await;
                                                     let _ = swarm_sender.send(Command::PublishPipeline(next_event)).await;
                                                 });
                                             },
                                             PipelineEvent::ForwardPass { session_id, layer_start, tensor: _ } => {
                                                 if layer_start < 30 {
                                                     // Continue the chain
                                                     let next_layer = layer_start + 10;
                                                     let next_event = if next_layer >= 30 {
                                                         PipelineEvent::Result { 
                                                             session_id, 
                                                             token: "Hello xNet!".to_string() 
                                                         }
                                                     } else {
                                                         PipelineEvent::ForwardPass {
                                                             session_id,
                                                             layer_start: next_layer,
                                                             tensor: xnet_core::Tensor { shape: vec![1, 4096], data: vec![0.2; 10] }
                                                         }
                                                     };
                                                     
                                                     // Broadcast after a delay
                                                     let swarm_sender = sender.clone();
                                                     tokio::spawn(async move {
                                                         tokio::time::sleep(Duration::from_secs(2)).await;
                                                         let _ = swarm_sender.send(Command::PublishPipeline(next_event)).await;
                                                     });
                                                 }
                                             },
                                             _ => {}
                                         }
                                     }
                                 } else if topic == "xnet/verification/v1" {
                                      // Handle Verification Event
                                     if let Ok(event) = serde_json::from_slice::<VerificationEvent>(&message.data) {
                                         println!("Got verification event from {}: {:?}", peer_id, event);
                                         let _ = event_sender_clone.send(NetworkEvent::VerificationEvent(event));
                                     }
                                 } else if topic == "xnet/fl/v1" {
                                     // Handle FL Event
                                     if let Ok(event) = serde_json::from_slice::<FLEvent>(&message.data) {
                                         println!("Got FL event from {}: {:?}", peer_id, event);
                                         let _ = event_sender_clone.send(NetworkEvent::FLEvent(event));
                                     }
                                 }
                                 
                                 let text = String::from_utf8_lossy(&message.data);
                                 let _ = event_sender_clone.send(NetworkEvent::Message(format!("[{}] {}", topic, text)));
                             },
                             libp2p::swarm::SwarmEvent::Behaviour(RhizomeBehaviourEvent::Kad(kad::Event::ModeChanged { new_mode })) => {
                                 println!("Kademlia mode changed: {:?}", new_mode);
                                 let _ = event_sender_clone.send(NetworkEvent::DhtEvent(format!("Mode: {:?}", new_mode)));
                             },
                             libp2p::swarm::SwarmEvent::Behaviour(RhizomeBehaviourEvent::Kad(kad::Event::RoutingUpdated { peer, .. })) => {
                                 println!("DHT Routing updated: {:?}", peer);
                                 let _ = event_sender_clone.send(NetworkEvent::DhtEvent(format!("Route Added: {}", peer)));
                             },
                             // Catch-all for other Kademlia events to avoid noise
                             libp2p::swarm::SwarmEvent::Behaviour(RhizomeBehaviourEvent::Kad(_)) => {},
                             _ => {}
                        }
                    }
                    command = receiver.recv() => {
                        match command {
                            Some(Command::PublishTask(task)) => {
                                if let Ok(data) = serde_json::to_vec(&task) {
                                     let topic = gossipsub::IdentTopic::new("xnet/tasks/v1");
                                     if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic, data) {
                                         println!("Publish error: {:?}", e);
                                     }
                                }
                            }
                            Some(Command::PublishPipeline(event)) => {
                                // 1. Publish to Network
                                if let Ok(data) = serde_json::to_vec(&event) {
                                    let topic = gossipsub::IdentTopic::new("xnet/pipeline/v1");
                                    if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic, data) {
                                        println!("Publish pipeline error: {:?}", e);
                                    }
                                }
                                
                                // 2. Loopback & Simulation (Handle event locally as if received)
                                let _ = event_sender_clone.send(NetworkEvent::PipelineEvent(event.clone()));

                                match event {
                                     PipelineEvent::InitSession { session_id, model: _ } => {
                                         // Start the chain! Layer 0-10
                                         let next_event = PipelineEvent::ForwardPass {
                                             session_id,
                                             layer_start: 0,
                                             tensor: xnet_core::Tensor { shape: vec![1, 4096], data: vec![0.1; 10] }
                                         };
                                         // Broadcast after a delay (simulating compute)
                                         let swarm_sender = sender.clone();
                                         tokio::spawn(async move {
                                             tokio::time::sleep(Duration::from_secs(2)).await;
                                             let _ = swarm_sender.send(Command::PublishPipeline(next_event)).await;
                                         });
                                     },
                                     PipelineEvent::ForwardPass { session_id, layer_start, tensor: _ } => {
                                         if layer_start < 30 {
                                             // Continue the chain
                                             let next_layer = layer_start + 10;
                                             let next_event = if next_layer >= 30 {
                                                 PipelineEvent::Result { 
                                                     session_id, 
                                                     token: "Hello xNet!".to_string() 
                                                 }
                                             } else {
                                                 PipelineEvent::ForwardPass {
                                                     session_id,
                                                     layer_start: next_layer,
                                                     tensor: xnet_core::Tensor { shape: vec![1, 4096], data: vec![0.2; 10] }
                                                 }
                                             };
                                             
                                             // Broadcast after a delay
                                             let swarm_sender = sender.clone();
                                             tokio::spawn(async move {
                                                 tokio::time::sleep(Duration::from_secs(2)).await;
                                                 let _ = swarm_sender.send(Command::PublishPipeline(next_event)).await;
                                             });
                                         }
                                     },
                                     _ => {}
                                 }
                            }
                            Some(Command::PublishVerification(event)) => {
                                if let Ok(data) = serde_json::to_vec(&event) {
                                     let topic = gossipsub::IdentTopic::new("xnet/verification/v1");
                                     if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic, data) {
                                         println!("Publish verification error: {:?}", e);
                                     }
                                }
                                // Loopback
                                let _ = event_sender_clone.send(NetworkEvent::VerificationEvent(event.clone()));
                                
                                // Simulation: Respond to Challenge
                                if let VerificationEvent::ChallengeIssued(challenge) = event {
                                    let vote = xnet_core::Vote {
                                        session_id: challenge.target_session_id,
                                        voter_id: peer_id.to_string(),
                                        vote: xnet_core::VoteType::Valid,
                                    };
                                    let next_event = VerificationEvent::VoteCast(vote);
                                    let swarm_sender = sender.clone();
                                     tokio::spawn(async move {
                                         tokio::time::sleep(Duration::from_secs(1)).await;
                                         let _ = swarm_sender.send(Command::PublishVerification(next_event)).await;
                                     });
                                }
                            }
                            Some(Command::PublishFL(event)) => {
                                if let Ok(data) = serde_json::to_vec(&event) {
                                     let topic = gossipsub::IdentTopic::new("xnet/fl/v1");
                                     if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic, data) {
                                         println!("Publish FL error: {:?}", e);
                                     }
                                }
                                // Loopback
                                let _ = event_sender_clone.send(NetworkEvent::FLEvent(event));
                            }
                            Some(Command::StartProviding) => {
                                let key = kad::RecordKey::new(&b"xnet-provider-v1".to_vec());
                                println!("Announcing provider capability for xnet-provider-v1");
                                if let Err(e) = swarm.behaviour_mut().kad.start_providing(key) {
                                    println!("Failed to start providing: {:?}", e);
                                }
                            }
                            None => break,
                        }
                    }
                }
            }
            Ok::<(), anyhow::Error>(())
        });

        Ok(Self { sender, event_sender })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<NetworkEvent> {
        self.event_sender.subscribe()
    }
}

#[async_trait]
impl NetworkInterface for P2PNode {
    async fn publish_task(&self, task: InferenceTask) -> Result<(), DynError> {
        self.sender.send(Command::PublishTask(task)).await
            .map_err(|e| Box::new(e) as DynError)
    }

    async fn announce_provider(&self) -> Result<(), DynError> {
        self.sender.send(Command::StartProviding).await
            .map_err(|e| Box::new(e) as DynError)
    }

    async fn publish_pipeline_event(&self, event: PipelineEvent) -> Result<(), DynError> {
         self.sender.send(Command::PublishPipeline(event)).await
             .map_err(|e| Box::new(e) as DynError)
    }

    async fn publish_verification_event(&self, event: VerificationEvent) -> Result<(), DynError> {
        self.sender.send(Command::PublishVerification(event)).await
            .map_err(|e| Box::new(e) as DynError)
    }

    async fn publish_fl_event(&self, event: FLEvent) -> Result<(), DynError> {
        self.sender.send(Command::PublishFL(event)).await
            .map_err(|e| Box::new(e) as DynError)
    }
}

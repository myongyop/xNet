use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl NodeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Processing,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceTask {
    pub id: String,
    pub model_name: String,
    pub prompt: String,
    pub status: TaskStatus,
}

impl InferenceTask {
    pub fn new(id: impl Into<String>, model: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            model_name: model.into(),
            prompt: prompt.into(),
            status: TaskStatus::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeMetrics {
    pub uptime_seconds: u64,
    pub tasks_processed: u64,
    pub tasks_relayed: u64,
    pub credits: f64,
}

impl NodeMetrics {
    pub fn new() -> Self {
        Self::default()
    }
}

// Distributed Inference Types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tensor {
    pub shape: Vec<usize>,
    pub data: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineEvent {
    InitSession { session_id: String, model: String },
    ForwardPass { session_id: String, layer_start: usize, tensor: Tensor },
    Result { session_id: String, token: String },
    Error { session_id: String, error: String },
}

// Verification System Types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    pub target_session_id: String,
    pub target_layer: usize,
    pub challenger_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VoteType {
    Valid,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub session_id: String,
    pub voter_id: String,
    pub vote: VoteType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerificationEvent {
    ChallengeIssued(Challenge),
    VoteCast(Vote),
    SlashingEnforced { target_node_id: String, reason: String },
}

// Federated Learning Types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FLTask {
    pub id: String,
    pub model_id: String,
    pub round: u32,
    pub hyperparameters: String, // Simplified for demo
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FLUpdate {
    pub task_id: String,
    pub node_id: String,
    pub round: u32,
    pub gradients: Vec<f32>, // Simulated gradients
    pub metrics: String,     // e.g., "loss: 0.05"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FLEvent {
    GlobalModelUpdate(FLTask),
    LocalUpdate(FLUpdate),
}

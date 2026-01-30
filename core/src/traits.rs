use async_trait::async_trait;
use crate::types::{InferenceTask, PipelineEvent};
use std::error::Error;


pub type DynError = Box<dyn Error + Send + Sync>;

#[async_trait]
pub trait NetworkInterface: Send + Sync {
    async fn publish_task(&self, task: InferenceTask) -> Result<(), DynError>;
    async fn announce_provider(&self) -> Result<(), DynError>;
    async fn publish_pipeline_event(&self, event: crate::types::PipelineEvent) -> Result<(), DynError>;
    async fn publish_verification_event(&self, event: crate::types::VerificationEvent) -> Result<(), DynError>;
    async fn publish_fl_event(&self, event: crate::types::FLEvent) -> Result<(), DynError>;
}

#[async_trait]
pub trait RuntimeInterface: Send + Sync {
    async fn generate(&self, model: &str, prompt: &str) -> Result<String, DynError>;
}

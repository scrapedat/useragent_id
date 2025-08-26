use async_trait::async_trait;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use crate::types::{Task, PlannerError, ExecutionTrace};
use crate::circuit::CircuitProtected;
use crate::client::LaVagueClient;

/// Core Actor system for the planner that defines the actor behavior
#[async_trait]
pub trait Actor: Send + 'static {
    type Message: Send + 'static;
    
    async fn handle(&mut self, msg: Self::Message);
    async fn pre_start(&mut self) {}
    async fn post_stop(&mut self) {}
}

/// Handle to send messages to an actor
pub struct ActorHandle<M: Send + 'static> {
    sender: mpsc::Sender<M>,
}

impl<M: Send + 'static> ActorHandle<M> {
    pub fn new(sender: mpsc::Sender<M>) -> Self {
        Self { sender }
    }
    
    pub async fn send(&self, msg: M) -> Result<(), mpsc::error::SendError<M>> {
        self.sender.send(msg).await
    }
    
    pub fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

/// Spawns an actor in a tokio task and returns a handle to it
pub async fn spawn_actor<A, M>(mut actor: A) -> ActorHandle<M> 
where
    A: Actor<Message = M>,
    M: Send + 'static,
{
    let (sender, mut receiver) = mpsc::channel::<M>(32);
    
    tokio::spawn(async move {
        actor.pre_start().await;
        
        while let Some(msg) = receiver.recv().await {
            actor.handle(msg).await;
        }
        
        actor.post_stop().await;
    });
    
    ActorHandle::new(sender)
}

/// Actor that handles LaVauge API communication
pub struct LaVagueActor {
    client: crate::client::LaVagueClient,
    timeout: Duration,
}

impl LaVagueActor {
    pub fn new(client: crate::client::LaVagueClient, timeout: Duration) -> Self {
        Self { 
            client,
            timeout: timeout.max(Duration::from_secs(5)), // Ensure minimum timeout
        }
    }
}

/// Messages that can be sent to the LaVauge actor
#[derive(Debug)]
pub enum LaVagueMessage {
    DecomposeTask {
        objective: String, 
        context_keys: Vec<String>,
        respond_to: oneshot::Sender<Result<Task, PlannerError>>,
    },
    SubmitFeedback {
        trace: ExecutionTrace,
        respond_to: oneshot::Sender<Result<(), PlannerError>>,
    },
}

#[async_trait]
impl Actor for LaVagueActor {
    type Message = LaVagueMessage;
    
    async fn handle(&mut self, msg: Self::Message) {
        match msg {
            LaVagueMessage::DecomposeTask { objective, context_keys, respond_to } => {
                let result = tokio::time::timeout(
                    self.timeout,
                    self.client.decompose_task(&objective, &context_keys)
                ).await;
                
                // Convert timeout to error if needed
                let result = match result {
                    Ok(r) => r,
                    Err(_) => Err(PlannerError::Timeout),
                };
                
                // Ignore send error - receiver may have dropped
                let _ = respond_to.send(result);
            },
            LaVagueMessage::SubmitFeedback { trace, respond_to } => {
                let result = tokio::time::timeout(
                    self.timeout,
                    self.client.submit_feedback(&trace)
                ).await;
                
                // Convert timeout to error if needed
                let result = match result {
                    Ok(r) => r,
                    Err(_) => Err(PlannerError::Timeout),
                };
                
                // Ignore send error - receiver may have dropped
                let _ = respond_to.send(result);
            },
        }
    }
    
    async fn pre_start(&mut self) {
        // Initialize any resources needed by the actor
        log::info!("LaVagueActor started");
    }
    
    async fn post_stop(&mut self) {
        // Clean up any resources
        log::info!("LaVagueActor stopped");
    }
}

/// Actor system for the planner component
pub struct PlannerActorSystem {
    lavague: ActorHandle<LaVagueMessage>,
}

impl PlannerActorSystem {
    /// Create and initialize the actor system
    pub async fn new(client: crate::client::LaVagueClient) -> Self {
        let lavague_actor = LaVagueActor::new(client, Duration::from_secs(30));
        let lavague = spawn_actor(lavague_actor).await;
        
        Self { lavague }
    }
    
    /// Access to the LaVague actor
    pub fn lavague(&self) -> ActorHandle<LaVagueMessage> {
        self.lavague.clone()
    }
    
    /// Decompose a task using the LaVague actor
    pub async fn decompose_task(&self, objective: String, context_keys: Vec<String>) -> Result<Task, PlannerError> {
        let (sender, receiver) = oneshot::channel();
        
        self.lavague.send(LaVagueMessage::DecomposeTask {
            objective,
            context_keys,
            respond_to: sender,
        }).await.map_err(|_| PlannerError::ActorError("LaVague actor unavailable".to_string()))?;
        
        receiver.await.map_err(|_| PlannerError::ResponseChannelClosed("Response channel closed".to_string()))?
    }
    
    /// Submit execution feedback using the LaVague actor
    pub async fn submit_feedback(&self, trace: ExecutionTrace) -> Result<(), PlannerError> {
        let (sender, receiver) = oneshot::channel();
        
        self.lavague.send(LaVagueMessage::SubmitFeedback {
            trace,
            respond_to: sender,
        }).await.map_err(|_| PlannerError::ActorError("LaVague actor unavailable".to_string()))?;
        
        receiver.await.map_err(|_| PlannerError::ActorError("Response channel closed".to_string()))?
    }
}

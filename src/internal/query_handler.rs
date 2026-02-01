//! Query handler for bidirectional control protocol.

#![allow(dead_code)]

use futures::Stream;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};

use crate::error::{ClaudeSDKError, Result};
use crate::transport::Transport;
use crate::types::{
    ControlResponseVariant, HookContext, HookEvent, HookInput, HookJSONOutput, HookMatcher,
    Message, PermissionResult, SDKControlRequest, SDKControlRequestVariant, SDKControlResponse,
    ToolPermissionContext,
};

use super::message_parser::parse_message;

/// Type alias for the tool permission callback function.
pub type CanUseToolFn = Arc<
    dyn Fn(
            String,
            Value,
            ToolPermissionContext,
        ) -> Pin<Box<dyn Future<Output = PermissionResult> + Send>>
        + Send
        + Sync,
>;

/// Type alias for hook callback function.
pub type HookCallbackFn = Arc<
    dyn Fn(
            HookInput,
            Option<String>,
            HookContext,
        ) -> Pin<Box<dyn Future<Output = HookJSONOutput> + Send>>
        + Send
        + Sync,
>;

/// Query handler that manages bidirectional control protocol on top of Transport.
pub struct QueryHandler {
    transport: Box<dyn Transport>,
    is_streaming_mode: bool,
    can_use_tool: Option<CanUseToolFn>,
    hooks: HashMap<HookEvent, Vec<HookMatcher>>,

    // Control protocol state
    pending_responses: Arc<Mutex<HashMap<String, oneshot::Sender<Result<Value>>>>>,
    hook_callbacks: Arc<Mutex<HashMap<String, HookCallbackFn>>>,
    request_counter: AtomicU64,
    next_callback_id: AtomicU64,

    // Message channel
    message_tx: Option<mpsc::Sender<Result<Message>>>,
    message_rx: Option<mpsc::Receiver<Result<Message>>>,

    // State
    initialized: bool,
    initialization_result: Option<Value>,
    initialize_timeout_secs: u64,
}

impl QueryHandler {
    /// Create a new query handler.
    pub fn new(
        transport: Box<dyn Transport>,
        is_streaming_mode: bool,
        can_use_tool: Option<CanUseToolFn>,
        hooks: HashMap<HookEvent, Vec<HookMatcher>>,
        initialize_timeout_secs: u64,
    ) -> Self {
        let (message_tx, message_rx) = mpsc::channel(100);

        Self {
            transport,
            is_streaming_mode,
            can_use_tool,
            hooks,
            pending_responses: Arc::new(Mutex::new(HashMap::new())),
            hook_callbacks: Arc::new(Mutex::new(HashMap::new())),
            request_counter: AtomicU64::new(0),
            next_callback_id: AtomicU64::new(0),
            message_tx: Some(message_tx),
            message_rx: Some(message_rx),
            initialized: false,
            initialization_result: None,
            initialize_timeout_secs,
        }
    }

    /// Initialize control protocol if in streaming mode.
    pub async fn initialize(&mut self) -> Result<Option<Value>> {
        if !self.is_streaming_mode {
            return Ok(None);
        }

        // Build hooks configuration
        let hooks_config = self.build_hooks_config().await;

        let request = SDKControlRequestVariant::Initialize {
            hooks: if hooks_config.is_empty() {
                None
            } else {
                Some(hooks_config)
            },
        };

        let response = self
            .send_control_request(request, self.initialize_timeout_secs)
            .await?;

        self.initialized = true;
        self.initialization_result = Some(response.clone());
        Ok(Some(response))
    }

    /// Build hooks configuration for initialization.
    async fn build_hooks_config(&mut self) -> HashMap<String, Value> {
        let mut hooks_config = HashMap::new();

        for (event, matchers) in &self.hooks {
            if matchers.is_empty() {
                continue;
            }

            let mut event_matchers = Vec::new();

            for matcher in matchers {
                let mut callback_ids = Vec::new();

                for hook in &matcher.hooks {
                    let callback_id = format!(
                        "hook_{}",
                        self.next_callback_id.fetch_add(1, Ordering::SeqCst)
                    );
                    self.hook_callbacks
                        .lock()
                        .await
                        .insert(callback_id.clone(), hook.clone());
                    callback_ids.push(callback_id);
                }

                let mut matcher_config = json!({
                    "matcher": matcher.matcher,
                    "hookCallbackIds": callback_ids,
                });

                if let Some(timeout) = matcher.timeout {
                    matcher_config["timeout"] = json!(timeout);
                }

                event_matchers.push(matcher_config);
            }

            hooks_config.insert(event.as_str().to_string(), json!(event_matchers));
        }

        hooks_config
    }

    /// Start reading messages from transport.
    pub async fn start(&mut self) -> Result<()> {
        // This would typically spawn a background task to read messages
        // For simplicity, we'll handle it in the receive_messages method
        Ok(())
    }

    /// Send a control request and wait for response.
    async fn send_control_request(
        &mut self,
        request: SDKControlRequestVariant,
        timeout_secs: u64,
    ) -> Result<Value> {
        if !self.is_streaming_mode {
            return Err(ClaudeSDKError::ControlProtocol(
                "Control requests require streaming mode".to_string(),
            ));
        }

        let request_id = format!(
            "req_{}_{}",
            self.request_counter.fetch_add(1, Ordering::SeqCst),
            rand_hex()
        );

        let (tx, rx) = oneshot::channel();
        self.pending_responses
            .lock()
            .await
            .insert(request_id.clone(), tx);

        let control_request = SDKControlRequest::new(request_id.clone(), request);
        let json_str = serde_json::to_string(&control_request)?;

        self.transport.write(&format!("{}\n", json_str)).await?;

        // Wait for response with timeout
        let result = tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), rx)
            .await
            .map_err(|_| {
                ClaudeSDKError::Timeout(format!(
                    "Control request timed out after {} seconds",
                    timeout_secs
                ))
            })?
            .map_err(|_| {
                ClaudeSDKError::ControlProtocol("Response channel closed".to_string())
            })??;

        Ok(result)
    }

    /// Handle an incoming control request from CLI.
    async fn handle_control_request(
        &self,
        _request_id: String,
        request: SDKControlRequestVariant,
    ) -> Result<Value> {
        match request {
            SDKControlRequestVariant::CanUseTool {
                tool_name,
                input,
                permission_suggestions,
                ..
            } => {
                let can_use_tool = self.can_use_tool.as_ref().ok_or_else(|| {
                    ClaudeSDKError::ControlProtocol(
                        "canUseTool callback is not provided".to_string(),
                    )
                })?;

                let _suggestions = permission_suggestions.unwrap_or_default();
                let context = ToolPermissionContext {
                    signal: None,
                    suggestions: Vec::new(), // TODO: Parse suggestions
                };

                let original_input = input.clone();
                let result = can_use_tool(tool_name.clone(), input, context).await;

                let response = match result {
                    PermissionResult::Allow(allow) => {
                        let mut resp = json!({
                            "behavior": "allow",
                            "updatedInput": allow.updated_input.unwrap_or(original_input),
                        });
                        if let Some(permissions) = allow.updated_permissions {
                            let perm_dicts: Vec<_> = permissions
                                .iter()
                                .map(|p| serde_json::to_value(p.to_dict()).unwrap_or_default())
                                .collect();
                            resp["updatedPermissions"] = json!(perm_dicts);
                        }
                        resp
                    }
                    PermissionResult::Deny(deny) => {
                        let mut resp = json!({
                            "behavior": "deny",
                            "message": deny.message,
                        });
                        if deny.interrupt {
                            resp["interrupt"] = json!(true);
                        }
                        resp
                    }
                };

                Ok(response)
            }

            SDKControlRequestVariant::HookCallback {
                callback_id,
                input,
                tool_use_id,
            } => {
                let callbacks = self.hook_callbacks.lock().await;
                let callback = callbacks.get(&callback_id).ok_or_else(|| {
                    ClaudeSDKError::ControlProtocol(format!(
                        "No hook callback found for ID: {}",
                        callback_id
                    ))
                })?;

                // Parse input into HookInput
                let hook_input: HookInput = serde_json::from_value(input)?;
                let context = HookContext { signal: None };

                let output = callback.clone()(hook_input, tool_use_id, context).await;

                // Convert to CLI format (async_ -> async, continue_ -> continue)
                let output_value = serde_json::to_value(&output)?;
                Ok(output_value)
            }

            SDKControlRequestVariant::McpMessage {
                server_name,
                message,
            } => {
                // TODO: Implement MCP server routing
                Ok(json!({
                    "jsonrpc": "2.0",
                    "id": message.get("id"),
                    "error": {
                        "code": -32601,
                        "message": format!("Server '{}' not found", server_name)
                    }
                }))
            }

            _ => Err(ClaudeSDKError::ControlProtocol(format!(
                "Unsupported control request: {:?}",
                request
            ))),
        }
    }

    /// Send interrupt signal.
    pub async fn interrupt(&mut self) -> Result<()> {
        self.send_control_request(SDKControlRequestVariant::Interrupt, 60)
            .await?;
        Ok(())
    }

    /// Change permission mode.
    pub async fn set_permission_mode(&mut self, mode: &str) -> Result<()> {
        self.send_control_request(
            SDKControlRequestVariant::SetPermissionMode {
                mode: mode.to_string(),
            },
            60,
        )
        .await?;
        Ok(())
    }

    /// Change the AI model.
    pub async fn set_model(&mut self, model: Option<String>) -> Result<()> {
        self.send_control_request(SDKControlRequestVariant::SetModel { model }, 60)
            .await?;
        Ok(())
    }

    /// Rewind tracked files to their state at a specific user message.
    pub async fn rewind_files(&mut self, user_message_id: &str) -> Result<()> {
        self.send_control_request(
            SDKControlRequestVariant::RewindFiles {
                user_message_id: user_message_id.to_string(),
            },
            60,
        )
        .await?;
        Ok(())
    }

    /// Get MCP server status.
    pub async fn get_mcp_status(&mut self) -> Result<Value> {
        self.send_control_request(SDKControlRequestVariant::McpStatus, 60)
            .await
    }

    /// Write data to transport.
    pub async fn write(&mut self, data: &str) -> Result<()> {
        self.transport.write(data).await
    }

    /// End input stream.
    pub async fn end_input(&mut self) -> Result<()> {
        self.transport.end_input().await
    }

    /// Receive messages from the transport.
    ///
    /// Note: This simplified version does not handle bidirectional control requests
    /// within the stream. For full bidirectional support, use a channel-based approach.
    pub fn receive_messages(&mut self) -> impl Stream<Item = Result<Message>> + '_ {
        let pending_responses = self.pending_responses.clone();

        async_stream::try_stream! {
            let mut stream = self.transport.read_messages();

            while let Some(result) = futures::StreamExt::next(&mut stream).await {
                let data = result?;

                let msg_type = data.get("type").and_then(|v| v.as_str());

                match msg_type {
                    Some("control_response") => {
                        // Route control response to pending request
                        if let Ok(response) = serde_json::from_value::<SDKControlResponse>(data.clone()) {
                            let request_id = response.request_id().to_string();
                            let mut pending = pending_responses.lock().await;

                            if let Some(tx) = pending.remove(&request_id) {
                                let result = match response.response {
                                    ControlResponseVariant::Success { response, .. } => {
                                        Ok(response.unwrap_or(Value::Null))
                                    }
                                    ControlResponseVariant::Error { error, .. } => {
                                        Err(ClaudeSDKError::ControlProtocol(error))
                                    }
                                };
                                let _ = tx.send(result);
                            }
                        }
                        continue;
                    }

                    Some("control_request") | Some("control_cancel_request") => {
                        // Skip control requests in simplified mode
                        // Full bidirectional support requires channel-based architecture
                        continue;
                    }

                    _ => {
                        // Regular SDK message
                        let message = parse_message(data)?;
                        yield message;
                    }
                }
            }
        }
    }

    /// Close the query handler and transport.
    pub async fn close(&mut self) -> Result<()> {
        self.transport.close().await
    }

    /// Get initialization result.
    pub fn initialization_result(&self) -> Option<&Value> {
        self.initialization_result.as_ref()
    }
}

/// Handle a control request (static version for use in async closures).
async fn handle_control_request_static(
    request: &SDKControlRequestVariant,
    can_use_tool: &Option<CanUseToolFn>,
    hook_callbacks: &Arc<Mutex<HashMap<String, HookCallbackFn>>>,
) -> Result<Value> {
    match request {
        SDKControlRequestVariant::CanUseTool {
            tool_name,
            input,
            permission_suggestions: _,
            ..
        } => {
            let can_use_tool = can_use_tool.as_ref().ok_or_else(|| {
                ClaudeSDKError::ControlProtocol("canUseTool callback is not provided".to_string())
            })?;

            let context = ToolPermissionContext {
                signal: None,
                suggestions: Vec::new(),
            };

            let original_input = input.clone();
            let result = can_use_tool(tool_name.clone(), input.clone(), context).await;

            let response = match result {
                PermissionResult::Allow(allow) => {
                    let mut resp = json!({
                        "behavior": "allow",
                        "updatedInput": allow.updated_input.unwrap_or(original_input),
                    });
                    if let Some(permissions) = allow.updated_permissions {
                        let perm_dicts: Vec<_> = permissions
                            .iter()
                            .map(|p| serde_json::to_value(p.to_dict()).unwrap_or_default())
                            .collect();
                        resp["updatedPermissions"] = json!(perm_dicts);
                    }
                    resp
                }
                PermissionResult::Deny(deny) => {
                    let mut resp = json!({
                        "behavior": "deny",
                        "message": deny.message,
                    });
                    if deny.interrupt {
                        resp["interrupt"] = json!(true);
                    }
                    resp
                }
            };

            Ok(response)
        }

        SDKControlRequestVariant::HookCallback {
            callback_id,
            input,
            tool_use_id,
        } => {
            let callbacks = hook_callbacks.lock().await;
            let callback = callbacks.get(callback_id).ok_or_else(|| {
                ClaudeSDKError::ControlProtocol(format!(
                    "No hook callback found for ID: {}",
                    callback_id
                ))
            })?;

            let hook_input: HookInput = serde_json::from_value(input.clone())?;
            let context = HookContext { signal: None };

            let output = callback.clone()(hook_input, tool_use_id.clone(), context).await;
            let output_value = serde_json::to_value(&output)?;
            Ok(output_value)
        }

        SDKControlRequestVariant::McpMessage {
            server_name,
            message,
        } => Ok(json!({
            "jsonrpc": "2.0",
            "id": message.get("id"),
            "error": {
                "code": -32601,
                "message": format!("Server '{}' not found", server_name)
            }
        })),

        _ => Err(ClaudeSDKError::ControlProtocol(format!(
            "Unsupported control request: {:?}",
            request
        ))),
    }
}

/// Generate a random hex string.
fn rand_hex() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:x}", duration.subsec_nanos())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rand_hex() {
        let hex1 = rand_hex();
        let _hex2 = rand_hex();
        // Should be non-empty
        assert!(!hex1.is_empty());
        // Might be different (depends on timing)
    }
}

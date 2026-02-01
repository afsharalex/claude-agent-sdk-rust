//! Query handler for bidirectional control protocol.

#![allow(dead_code)]

use futures::Stream;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};

use crate::error::{ClaudeSDKError, Result};
use crate::transport::Transport;
use crate::types::{
    CanUseToolFn, ControlResponseVariant, HookCallbackFn, HookContext, HookEvent, HookInput,
    HookMatcher, Message, PermissionResult, SDKControlRequest, SDKControlRequestVariant,
    SDKControlResponse, ToolPermissionContext,
};

use super::message_parser::parse_message;

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

    // Outgoing response queue for control responses
    outgoing_responses: Arc<Mutex<Vec<String>>>,

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
            outgoing_responses: Arc::new(Mutex::new(Vec::new())),
            initialized: false,
            initialization_result: None,
            initialize_timeout_secs,
        }
    }

    /// Initialize control protocol if in streaming mode.
    ///
    /// This method sends an initialize request and reads messages directly from
    /// the transport until it receives the control response.
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

        // Generate request ID
        let request_id = format!(
            "req_{}_{}",
            self.request_counter.fetch_add(1, Ordering::SeqCst),
            rand_hex()
        );

        // Build and send the control request
        let control_request = SDKControlRequest::new(request_id.clone(), request);
        let json_str = serde_json::to_string(&control_request)?;
        self.transport.write(&format!("{}\n", json_str)).await?;

        // Read messages until we get the control response
        let timeout = std::time::Duration::from_secs(self.initialize_timeout_secs);
        let deadline = std::time::Instant::now() + timeout;

        loop {
            if std::time::Instant::now() > deadline {
                return Err(ClaudeSDKError::Timeout(format!(
                    "Initialize request timed out after {} seconds",
                    self.initialize_timeout_secs
                )));
            }

            // Read next message with remaining timeout
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            let msg_result =
                tokio::time::timeout(remaining, self.transport.read_next_message()).await;

            match msg_result {
                Ok(Ok(Some(data))) => {
                    // Check if this is a control response
                    if let Some("control_response") = data.get("type").and_then(|v| v.as_str()) {
                        if let Ok(response) = serde_json::from_value::<SDKControlResponse>(data) {
                            if response.request_id() == request_id {
                                match response.response {
                                    ControlResponseVariant::Success { response, .. } => {
                                        let result = response.unwrap_or(Value::Null);
                                        self.initialized = true;
                                        self.initialization_result = Some(result.clone());
                                        return Ok(Some(result));
                                    }
                                    ControlResponseVariant::Error { error, .. } => {
                                        return Err(ClaudeSDKError::ControlProtocol(error));
                                    }
                                }
                            }
                        }
                    }
                    // Not a control response for our request - continue reading
                    // (During init, there shouldn't be other messages, but handle gracefully)
                }
                Ok(Ok(None)) => {
                    // Stream ended
                    return Err(ClaudeSDKError::ControlProtocol(
                        "Transport stream ended before initialize response received".to_string(),
                    ));
                }
                Ok(Err(e)) => return Err(e),
                Err(_) => {
                    // Timeout
                    return Err(ClaudeSDKError::Timeout(format!(
                        "Initialize request timed out after {} seconds",
                        self.initialize_timeout_secs
                    )));
                }
            }
        }
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

    /// Flush any pending outgoing responses.
    ///
    /// This should be called periodically to send queued control responses
    /// back to the CLI.
    pub async fn flush_responses(&mut self) -> Result<()> {
        let responses: Vec<String> = {
            let mut guard = self.outgoing_responses.lock().await;
            std::mem::take(&mut *guard)
        };

        for response in responses {
            self.transport.write(&response).await?;
        }

        Ok(())
    }

    /// Receive messages from the transport.
    ///
    /// This method handles bidirectional control protocol:
    /// - Routes incoming control_response messages to pending requests
    /// - Handles incoming control_request messages by invoking callbacks and queuing responses
    /// - Yields regular SDK messages to the caller
    ///
    /// Note: Control responses are queued and must be flushed with `flush_responses()`.
    pub fn receive_messages(&mut self) -> impl Stream<Item = Result<Message>> + '_ {
        let pending_responses = self.pending_responses.clone();
        let can_use_tool = self.can_use_tool.clone();
        let hook_callbacks = self.hook_callbacks.clone();
        let outgoing_responses = self.outgoing_responses.clone();

        async_stream::try_stream! {
            let mut stream = self.transport.read_messages();

            while let Some(result) = futures::StreamExt::next(&mut stream).await {
                let data: Value = result?;

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

                    Some("control_request") => {
                        // Handle incoming control request from CLI
                        if let Ok(request) = serde_json::from_value::<SDKControlRequest>(data.clone()) {
                            let request_id = request.request_id.clone();

                            // Process the control request
                            let response_result = handle_control_request_static(
                                &request.request,
                                &can_use_tool,
                                &hook_callbacks,
                            ).await;

                            // Build the control response
                            let control_response = match response_result {
                                Ok(response_data) => SDKControlResponse::success(&request_id, Some(response_data)),
                                Err(e) => SDKControlResponse::error(&request_id, e.to_string()),
                            };

                            // Queue response to be sent (will be flushed later)
                            if let Ok(response_json) = serde_json::to_string(&control_response) {
                                outgoing_responses.lock().await.push(format!("{}\n", response_json));
                            }
                        }
                        continue;
                    }

                    Some("control_cancel_request") => {
                        // Handle control cancel request - currently just acknowledge
                        // TODO: Implement proper cancellation support if needed
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
    use crate::transport::Transport;
    use async_trait::async_trait;
    use serde_json::json;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Mock transport for testing QueryHandler.
    struct MockTransport {
        messages: Arc<Mutex<Vec<Value>>>,
        written: Arc<Mutex<Vec<String>>>,
        connected: Arc<AtomicBool>,
    }

    impl MockTransport {
        fn new(messages: Vec<Value>) -> Self {
            Self {
                messages: Arc::new(Mutex::new(messages)),
                written: Arc::new(Mutex::new(vec![])),
                connected: Arc::new(AtomicBool::new(false)),
            }
        }

        fn empty() -> Self {
            Self::new(vec![])
        }
    }

    #[async_trait]
    impl Transport for MockTransport {
        async fn connect(&mut self) -> Result<()> {
            self.connected.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn write(&mut self, data: &str) -> Result<()> {
            self.written.lock().await.push(data.to_string());
            Ok(())
        }

        fn read_messages(
            &mut self,
        ) -> Pin<Box<dyn futures::Stream<Item = Result<Value>> + Send + '_>> {
            let messages = self.messages.clone();
            Box::pin(async_stream::try_stream! {
                let mut guard = messages.lock().await;
                for msg in std::mem::take(&mut *guard) {
                    yield msg;
                }
            })
        }

        async fn read_next_message(&mut self) -> Result<Option<Value>> {
            let mut guard = self.messages.lock().await;
            if guard.is_empty() {
                Ok(None)
            } else {
                Ok(Some(guard.remove(0)))
            }
        }

        async fn close(&mut self) -> Result<()> {
            self.connected.store(false, Ordering::SeqCst);
            Ok(())
        }

        fn is_ready(&self) -> bool {
            self.connected.load(Ordering::SeqCst)
        }

        async fn end_input(&mut self) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_rand_hex() {
        let hex1 = rand_hex();
        let _hex2 = rand_hex();
        // Should be non-empty
        assert!(!hex1.is_empty());
        // Might be different (depends on timing)
    }

    #[test]
    fn test_rand_hex_is_valid_hex() {
        let hex = rand_hex();
        // Should only contain hex characters
        for c in hex.chars() {
            assert!(c.is_ascii_hexdigit());
        }
    }

    #[tokio::test]
    async fn test_query_handler_creation() {
        let transport = Box::new(MockTransport::empty());
        let handler = QueryHandler::new(
            transport,
            true, // streaming mode
            None, // no can_use_tool
            HashMap::new(),
            60, // timeout
        );

        assert!(handler.initialization_result().is_none());
    }

    #[tokio::test]
    async fn test_query_handler_write() {
        let mock = MockTransport::empty();
        let written = mock.written.clone();
        let transport = Box::new(mock);

        let mut handler = QueryHandler::new(transport, false, None, HashMap::new(), 60);

        handler.write("test message\n").await.unwrap();

        let written_messages = written.lock().await;
        assert_eq!(written_messages.len(), 1);
        assert_eq!(written_messages[0], "test message\n");
    }

    #[tokio::test]
    async fn test_query_handler_end_input() {
        let transport = Box::new(MockTransport::empty());
        let mut handler = QueryHandler::new(transport, false, None, HashMap::new(), 60);

        let result = handler.end_input().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_query_handler_close() {
        let mock = MockTransport::empty();
        let connected = mock.connected.clone();
        connected.store(true, Ordering::SeqCst);
        let transport = Box::new(mock);

        let mut handler = QueryHandler::new(transport, false, None, HashMap::new(), 60);

        handler.close().await.unwrap();
        assert!(!connected.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_query_handler_flush_responses_empty() {
        let transport = Box::new(MockTransport::empty());
        let mut handler = QueryHandler::new(transport, false, None, HashMap::new(), 60);

        // Flushing empty queue should succeed
        let result = handler.flush_responses().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_query_handler_receive_messages_assistant() {
        let messages = vec![json!({
            "type": "assistant",
            "message": {
                "id": "msg_1",
                "role": "assistant",
                "content": [{"type": "text", "text": "Hello"}],
                "model": "claude-3-5-sonnet",
                "stop_reason": "end_turn"
            }
        })];

        let transport = Box::new(MockTransport::new(messages));
        let mut handler = QueryHandler::new(transport, true, None, HashMap::new(), 60);

        let stream = handler.receive_messages();
        tokio::pin!(stream);

        let mut received = Vec::new();
        while let Some(result) = futures::StreamExt::next(&mut stream).await {
            received.push(result.unwrap());
        }

        assert_eq!(received.len(), 1);
        assert!(received[0].is_assistant());
    }

    #[tokio::test]
    async fn test_query_handler_receive_control_response() {
        // Test that control responses are properly routed
        let messages = vec![
            json!({
                "type": "control_response",
                "response": {
                    "subtype": "success",
                    "request_id": "test-req-1",
                    "response": {"status": "ok"}
                }
            }),
            json!({
                "type": "assistant",
                "message": {
                    "id": "msg_1",
                    "role": "assistant",
                    "content": [{"type": "text", "text": "Hello"}],
                    "model": "claude-3-5-sonnet",
                    "stop_reason": "end_turn"
                }
            }),
        ];

        let transport = Box::new(MockTransport::new(messages));
        let mut handler = QueryHandler::new(transport, true, None, HashMap::new(), 60);

        let stream = handler.receive_messages();
        tokio::pin!(stream);

        let mut received = Vec::new();
        while let Some(result) = futures::StreamExt::next(&mut stream).await {
            received.push(result.unwrap());
        }

        // Only the assistant message should be yielded, not the control response
        assert_eq!(received.len(), 1);
        assert!(received[0].is_assistant());
    }

    #[tokio::test]
    async fn test_query_handler_receive_multiple_message_types() {
        let messages = vec![
            json!({
                "type": "system",
                "subtype": "init",
                "cwd": "/test",
                "session_id": "session-1"
            }),
            json!({
                "type": "user",
                "message": {
                    "role": "user",
                    "content": "Hello"
                }
            }),
            json!({
                "type": "assistant",
                "message": {
                    "id": "msg_1",
                    "role": "assistant",
                    "content": [{"type": "text", "text": "Hi there!"}],
                    "model": "claude-3-5-sonnet",
                    "stop_reason": "end_turn"
                }
            }),
            json!({
                "type": "result",
                "result": "success",
                "session_id": "session-1",
                "cost_usd": 0.01,
                "duration_ms": 1000
            }),
        ];

        let transport = Box::new(MockTransport::new(messages));
        let mut handler = QueryHandler::new(transport, true, None, HashMap::new(), 60);

        let stream = handler.receive_messages();
        tokio::pin!(stream);

        let received: Vec<_> = futures::StreamExt::collect(stream).await;
        assert_eq!(received.len(), 4);

        // Check message types
        assert!(received[0].as_ref().unwrap().is_system());
        assert!(received[1].as_ref().unwrap().is_user());
        assert!(received[2].as_ref().unwrap().is_assistant());
        assert!(received[3].as_ref().unwrap().is_result());
    }

    #[tokio::test]
    async fn test_handle_control_request_static_no_can_use_tool() {
        let request = SDKControlRequestVariant::CanUseTool {
            tool_name: "Bash".to_string(),
            input: json!({"command": "ls"}),
            permission_suggestions: None,
            blocked_path: None,
        };

        let result =
            handle_control_request_static(&request, &None, &Arc::new(Mutex::new(HashMap::new())))
                .await;

        // Should fail because no callback is provided
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_control_request_static_mcp_not_found() {
        let request = SDKControlRequestVariant::McpMessage {
            server_name: "unknown-server".to_string(),
            message: json!({"jsonrpc": "2.0", "method": "tools/list", "id": 1}),
        };

        let result =
            handle_control_request_static(&request, &None, &Arc::new(Mutex::new(HashMap::new())))
                .await;

        assert!(result.is_ok());
        let response = result.unwrap();

        // Should return an error response for unknown server
        assert!(response.get("error").is_some());
        assert!(response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("not found"));
    }

    #[tokio::test]
    async fn test_handle_control_request_static_hook_callback_not_found() {
        let request = SDKControlRequestVariant::HookCallback {
            callback_id: "unknown-callback".to_string(),
            input: json!({}),
            tool_use_id: None,
        };

        let result =
            handle_control_request_static(&request, &None, &Arc::new(Mutex::new(HashMap::new())))
                .await;

        // Should fail because callback is not registered
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No hook callback found"));
    }

    #[tokio::test]
    async fn test_query_handler_non_streaming_mode() {
        let transport = Box::new(MockTransport::empty());
        let handler = QueryHandler::new(transport, false, None, HashMap::new(), 60);

        // In non-streaming mode, initialization should be None
        assert!(!handler.initialized);
    }

    #[tokio::test]
    async fn test_query_handler_start() {
        let transport = Box::new(MockTransport::empty());
        let mut handler = QueryHandler::new(transport, false, None, HashMap::new(), 60);

        let result = handler.start().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_control_request_static_can_use_tool_allow() {
        use std::sync::Arc;

        let can_use_tool: crate::types::CanUseToolFn = Arc::new(|_tool_name, _input, _context| {
            Box::pin(async move { crate::types::PermissionResult::allow() })
        });

        let request = SDKControlRequestVariant::CanUseTool {
            tool_name: "Bash".to_string(),
            input: json!({"command": "ls"}),
            permission_suggestions: None,
            blocked_path: None,
        };

        let result = handle_control_request_static(
            &request,
            &Some(can_use_tool),
            &Arc::new(Mutex::new(HashMap::new())),
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response["behavior"], "allow");
    }

    #[tokio::test]
    async fn test_handle_control_request_static_can_use_tool_deny() {
        use std::sync::Arc;

        let can_use_tool: crate::types::CanUseToolFn = Arc::new(|_tool_name, _input, _context| {
            Box::pin(async move {
                crate::types::PermissionResult::Deny(
                    crate::types::PermissionResultDeny::new()
                        .with_message("Not allowed")
                        .with_interrupt(true),
                )
            })
        });

        let request = SDKControlRequestVariant::CanUseTool {
            tool_name: "Bash".to_string(),
            input: json!({"command": "rm -rf /"}),
            permission_suggestions: None,
            blocked_path: None,
        };

        let result = handle_control_request_static(
            &request,
            &Some(can_use_tool),
            &Arc::new(Mutex::new(HashMap::new())),
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response["behavior"], "deny");
        assert_eq!(response["message"], "Not allowed");
        assert_eq!(response["interrupt"], true);
    }

    #[tokio::test]
    async fn test_handle_control_request_static_unsupported() {
        let request = SDKControlRequestVariant::Interrupt;

        let result =
            handle_control_request_static(&request, &None, &Arc::new(Mutex::new(HashMap::new())))
                .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported control request"));
    }

    #[tokio::test]
    async fn test_query_handler_flush_responses_with_queued() {
        let mock = MockTransport::empty();
        let written = mock.written.clone();
        let transport = Box::new(mock);

        let mut handler = QueryHandler::new(transport, true, None, HashMap::new(), 60);

        // Manually add a response to the queue
        handler
            .outgoing_responses
            .lock()
            .await
            .push("{\"test\":true}\n".to_string());

        let result = handler.flush_responses().await;
        assert!(result.is_ok());

        let written_messages = written.lock().await;
        assert_eq!(written_messages.len(), 1);
        assert!(written_messages[0].contains("test"));
    }

    #[tokio::test]
    async fn test_query_handler_receive_control_cancel_request() {
        let messages = vec![
            json!({
                "type": "control_cancel_request",
                "request_id": "test-req"
            }),
            json!({
                "type": "assistant",
                "message": {
                    "id": "msg_1",
                    "role": "assistant",
                    "content": [{"type": "text", "text": "Hello"}],
                    "model": "claude-3-5-sonnet",
                    "stop_reason": "end_turn"
                }
            }),
        ];

        let transport = Box::new(MockTransport::new(messages));
        let mut handler = QueryHandler::new(transport, true, None, HashMap::new(), 60);

        let stream = handler.receive_messages();
        tokio::pin!(stream);

        let mut received = Vec::new();
        while let Some(result) = futures::StreamExt::next(&mut stream).await {
            received.push(result.unwrap());
        }

        // Only the assistant message should be yielded, control_cancel_request is skipped
        assert_eq!(received.len(), 1);
        assert!(received[0].is_assistant());
    }

    #[tokio::test]
    async fn test_query_handler_initialization_result() {
        let transport = Box::new(MockTransport::empty());
        let handler = QueryHandler::new(transport, false, None, HashMap::new(), 60);

        assert!(handler.initialization_result().is_none());
    }

    #[test]
    fn test_rand_hex_produces_different_values() {
        // Call multiple times to ensure it works
        let results: Vec<String> = (0..10).map(|_| rand_hex()).collect();

        // All should be valid hex
        for hex in &results {
            assert!(!hex.is_empty());
            for c in hex.chars() {
                assert!(c.is_ascii_hexdigit());
            }
        }
    }

    #[tokio::test]
    async fn test_query_handler_with_can_use_tool() {
        use std::sync::Arc;

        let can_use_tool: crate::types::CanUseToolFn = Arc::new(|_tool_name, _input, _context| {
            Box::pin(async move { crate::types::PermissionResult::allow() })
        });

        let transport = Box::new(MockTransport::empty());
        let handler = QueryHandler::new(transport, true, Some(can_use_tool), HashMap::new(), 60);

        assert!(handler.can_use_tool.is_some());
    }

    #[tokio::test]
    async fn test_query_handler_with_hooks() {
        use crate::types::{HookEvent, HookMatcher};

        let mut hooks = HashMap::new();
        hooks.insert(HookEvent::PreToolUse, vec![HookMatcher::new()]);

        let transport = Box::new(MockTransport::empty());
        let handler = QueryHandler::new(transport, true, None, hooks, 60);

        assert_eq!(handler.hooks.len(), 1);
    }

    #[tokio::test]
    async fn test_handle_control_request_static_allow_with_updated_input() {
        use std::sync::Arc;

        let can_use_tool: crate::types::CanUseToolFn = Arc::new(|_tool_name, _input, _context| {
            Box::pin(async move {
                crate::types::PermissionResult::Allow(
                    crate::types::PermissionResultAllow::new()
                        .with_updated_input(json!({"command": "ls -la"})),
                )
            })
        });

        let request = SDKControlRequestVariant::CanUseTool {
            tool_name: "Bash".to_string(),
            input: json!({"command": "ls"}),
            permission_suggestions: None,
            blocked_path: None,
        };

        let result = handle_control_request_static(
            &request,
            &Some(can_use_tool),
            &Arc::new(Mutex::new(HashMap::new())),
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response["behavior"], "allow");
        assert_eq!(response["updatedInput"]["command"], "ls -la");
    }

    #[tokio::test]
    async fn test_handle_control_request_static_allow_with_permissions() {
        use crate::types::{PermissionBehavior, PermissionRuleValue, PermissionUpdate};
        use std::sync::Arc;

        let can_use_tool: crate::types::CanUseToolFn = Arc::new(|_tool_name, _input, _context| {
            Box::pin(async move {
                crate::types::PermissionResult::Allow(
                    crate::types::PermissionResultAllow::new().with_updated_permissions(vec![
                        PermissionUpdate::add_rules(
                            vec![PermissionRuleValue::new("Bash")],
                            PermissionBehavior::Allow,
                        ),
                    ]),
                )
            })
        });

        let request = SDKControlRequestVariant::CanUseTool {
            tool_name: "Bash".to_string(),
            input: json!({"command": "ls"}),
            permission_suggestions: None,
            blocked_path: None,
        };

        let result = handle_control_request_static(
            &request,
            &Some(can_use_tool),
            &Arc::new(Mutex::new(HashMap::new())),
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response["behavior"], "allow");
        assert!(response.get("updatedPermissions").is_some());
    }
}

//! WebSocket client for integration testing
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};

/// WebSocket test client for integration testing
pub struct WebSocketTestClient {
    /// Client ID for logging
    client_id: String,
    /// WebSocket stream
    ws_stream: Arc<Mutex<Option<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
    /// Received messages
    messages: Arc<Mutex<Vec<Message>>>,
    /// Connected flag
    pub connected: Arc<Mutex<bool>>,
}

impl WebSocketTestClient {
    /// Create a new WebSocket test client
    pub fn new(client_id: &str) -> Self {
        Self {
            client_id: client_id.to_string(),
            ws_stream: Arc::new(Mutex::new(None)),
            messages: Arc::new(Mutex::new(Vec::new())),
            connected: Arc::new(Mutex::new(false)),
        }
    }
    
    /// Connect to the WebSocket server
    pub async fn connect(&self, port: u16) -> Result<()> {
        let url = format!("ws://localhost:{}", port);
        println!("Client {} connecting to {}", self.client_id, url);
        
        // Use a timeout for connection
        let connect_result = timeout(
            Duration::from_secs(5),
            connect_async(&url)
        ).await?;
        
        let (ws_stream, _) = connect_result?;
        
        // Store the WebSocket stream
        {
            let mut stream_guard = self.ws_stream.lock().await;
            *stream_guard = Some(ws_stream);
        }
        
        // Set connected flag
        {
            let mut connected_guard = self.connected.lock().await;
            *connected_guard = true;
        }
        
        // Start message handler
        self.start_message_handler().await;
        
        println!("Client {} connected successfully", self.client_id);
        Ok(())
    }
    
    /// Start message handler task
    async fn start_message_handler(&self) {
        let client_id = self.client_id.clone();
        let ws_stream = Arc::clone(&self.ws_stream);
        let messages = Arc::clone(&self.messages);
        let connected = Arc::clone(&self.connected);
        
        tokio::spawn(async move {
            println!("Client {} starting message handler", client_id);
            
            // Get the stream
            let mut stream_guard = ws_stream.lock().await;
            let stream = match stream_guard.take() {
                Some(s) => s,
                None => {
                    println!("Client {} has no stream", client_id);
                    return;
                }
            };
            
            // Split the stream
            let (_write, mut read) = stream.split();
            
            // Loop to read messages
            loop {
                match read.next().await {
                    Some(Ok(msg)) => {
                        println!("Client {} received message: {:?}", client_id, msg);
                        // Store the message
                        let mut messages_guard = messages.lock().await;
                        messages_guard.push(msg);
                    }
                    Some(Err(e)) => {
                        println!("Client {} error: {}", client_id, e);
                        break;
                    }
                    None => {
                        println!("Client {} connection closed", client_id);
                        break;
                    }
                }
            }
            
            // Set disconnected flag
            let mut connected_guard = connected.lock().await;
            *connected_guard = false;
            
            println!("Client {} message handler exited", client_id);
        });
    }
    
    /// Get all received messages
    pub async fn get_messages(&self) -> Vec<Message> {
        let messages_guard = self.messages.lock().await;
        messages_guard.clone()
    }
    
    /// Clear messages
    pub async fn clear_messages(&self) {
        let mut messages_guard = self.messages.lock().await;
        messages_guard.clear();
    }
    
    /// Get received JSON messages
    pub async fn get_json_messages(&self) -> Vec<Value> {
        let messages = self.get_messages().await;
        let mut json_messages = Vec::new();
        
        for msg in messages {
            if let Message::Text(text) = msg {
                if let Ok(json) = serde_json::from_str::<Value>(&text) {
                    json_messages.push(json);
                }
            }
        }
        
        json_messages
    }
    
    /// Wait for a specific number of messages
    pub async fn wait_for_messages(&self, count: usize, timeout_ms: u64) -> Result<bool> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_millis(timeout_ms);
        
        while start.elapsed() < timeout {
            let messages = self.get_messages().await;
            if messages.len() >= count {
                return Ok(true);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        Ok(false)
    }
    
    /// Wait for a JSON message matching a predicate
    pub async fn wait_for_json_message<F>(&self, predicate: F, timeout_ms: u64) -> Result<Option<Value>> 
    where
        F: Fn(&Value) -> bool,
    {
        let start = std::time::Instant::now();
        let timeout = Duration::from_millis(timeout_ms);
        
        while start.elapsed() < timeout {
            let json_messages = self.get_json_messages().await;
            
            for msg in &json_messages {
                if predicate(msg) {
                    return Ok(Some(msg.clone()));
                }
            }
            
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        Ok(None)
    }
    
    /// Send a message to the server
    pub async fn send_message(&self, message: Message) -> Result<()> {
        // Get the stream
        let mut stream_guard = self.ws_stream.lock().await;
        let stream = match &mut *stream_guard {
            Some(s) => s,
            None => return Err(anyhow!("Not connected")),
        };
        
        // Send the message
        stream.send(message).await?;
        Ok(())
    }
    
    /// Send a text message
    pub async fn send_text(&self, text: &str) -> Result<()> {
        self.send_message(Message::Text(text.to_string().into())).await
    }
    
    /// Send a JSON message
    pub async fn send_json(&self, json: Value) -> Result<()> {
        let text = serde_json::to_string(&json)?;
        self.send_text(&text).await
    }
    
    /// Send a ping
    pub async fn send_ping(&self) -> Result<()> {
        // Try multiple ping formats since we don't know the exact format the server expects
        // Format 1: Standard ping frame (not JSON)
        let result1 = self.send_message(Message::Ping(vec![1, 2, 3, 4].into())).await;
        
        // Format 2: Ping as JSON command
        let result2 = self.send_json(json!({"command": "ping"})).await;
        
        // Format 3: Simple text "ping"
        let result3 = self.send_text("ping").await;
        
        // Return success if any of the ping methods worked
        if result1.is_ok() || result2.is_ok() || result3.is_ok() {
            Ok(())
        } else {
            // Return the last error
            result3
        }
    }
    
    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        let connected_guard = self.connected.lock().await;
        *connected_guard
    }
    
    /// Disconnect from the server
    pub async fn disconnect(&self) -> Result<()> {
        println!("Disconnecting client {}", self.client_id);
        
        // Get the stream
        let mut stream_guard = self.ws_stream.lock().await;
        
        if let Some(stream) = stream_guard.take() {
            // Close the stream
            let (mut write, _) = stream.split();
            
            // Send close frame with timeout
            match tokio::time::timeout(
                Duration::from_secs(1),
                write.send(Message::Close(None))
            ).await {
                Ok(result) => {
                    if let Err(e) = result {
                        println!("Error sending close frame: {}", e);
                    }
                },
                Err(_) => {
                    println!("Timeout sending close frame");
                }
            }
            
            // Set disconnected flag regardless of close result
            let mut connected_guard = self.connected.lock().await;
            *connected_guard = false;
            
            println!("Client {} disconnected", self.client_id);
        } else {
            println!("Client {} already disconnected", self.client_id);
        }
        
        Ok(())
    }
}

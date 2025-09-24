use shared::{DownMsg, UpMsg};
use std::sync::Arc;
use zoon::*;

/// Actor+Relay compatible Connection adapter
#[derive(Clone)]
pub struct ConnectionAdapter {
    connection: Arc<SendWrapper<Connection<UpMsg, DownMsg>>>,
    // ✅ FIX: Keep message sender alive to prevent stream ending after Send bounds removal
    _message_sender: futures::channel::mpsc::UnboundedSender<DownMsg>,
}

impl ConnectionAdapter {
    pub fn new() -> (Self, impl futures::stream::Stream<Item = DownMsg>) {
        let (message_sender, message_stream) = futures::channel::mpsc::unbounded();

        // ✅ FIX: Clone sender to prevent closure capture dropping after Send bounds removal
        let sender_for_closure = message_sender.clone();
        let connection = Connection::new(move |down_msg, _| {
            let _ = sender_for_closure.unbounded_send(down_msg);
        });

        let adapter = ConnectionAdapter {
            connection: Arc::new(SendWrapper::new(connection)),
            // ✅ FIX: Store sender in struct to keep it alive
            _message_sender: message_sender,
        };
        (adapter, message_stream)
    }

    /// Create ConnectionAdapter from existing Arc<Connection>
    pub fn from_arc(connection: Arc<SendWrapper<Connection<UpMsg, DownMsg>>>) -> Self {
        // ✅ FIX: Create dummy sender for compatibility (not used when created from existing connection)
        let (dummy_sender, _) = futures::channel::mpsc::unbounded();
        ConnectionAdapter {
            connection,
            _message_sender: dummy_sender,
        }
    }

    pub async fn send_up_msg(&self, up_msg: UpMsg) {
        if let Err(error) = self.connection.send_up_msg(up_msg).await {
            zoon::println!("Failed to send message: {:?}", error);
        }
    }

    /// Get the underlying connection for platform initialization
    pub fn get_connection(&self) -> Arc<SendWrapper<Connection<UpMsg, DownMsg>>> {
        self.connection.clone()
    }
}

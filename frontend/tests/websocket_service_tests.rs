use frontend::services::websocket::*;
use std::rc::Rc;
use std::cell::RefCell;
use futures::{sink::Sink, task::{Context, Poll}};
use gloo_net::websocket::{Message, WebSocketError};
use std::pin::Pin;

struct MockSink {
    pub messages: Rc<RefCell<Vec<String>>>,
}

impl Sink<Message> for MockSink {
    type Error = WebSocketError;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        if let Message::Text(text) = item {
            self.messages.borrow_mut().push(text);
        }
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

#[tokio::test]
async fn test_flush_outgoing_buffer() {
    let messages = Rc::new(RefCell::new(Vec::new()));
    let mut sink = MockSink { messages: messages.clone() };
    let buffer = Rc::new(RefCell::new(vec!["msg1".to_string(), "msg2".to_string()]));
    
    WebSocketService::flush_outgoing_buffer(&mut sink, buffer.clone()).await.unwrap();
    
    assert_eq!(messages.borrow().len(), 2);
    assert_eq!(messages.borrow()[0], "msg1");
    assert_eq!(messages.borrow()[1], "msg2");
    assert!(buffer.borrow().is_empty());
}

#[test]
fn test_prepare_subscription_frame_basic() {
    let input = "SUBSCRIBE\ndestination:/topic/general\n\0".to_string();
    let (output, receipt_id) = prepare_subscription_frame(input);
    
    assert!(receipt_id.is_some());
    assert!(output.contains("SUBSCRIBE"));
    assert!(output.contains("destination:/topic/general"));
    assert!(output.contains("receipt:"));
}

#[test]
fn test_prepare_subscription_frame_with_seq() {
    let input = "SUBSCRIBE\ndestination:/topic/general\nlast_seen_seq:123\n\0".to_string();
    let (output, _receipt_id) = prepare_subscription_frame(input);
    
    assert!(output.contains("last_seen_seq:123"));
}

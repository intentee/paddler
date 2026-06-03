use crossterm::event::KeyEvent;
use crossterm::event::MouseEvent;
use paddler_messaging::inference_client::Message;

#[derive(Debug)]
pub enum ChatSessionEvent {
    InferenceMessage(Message),
    InferenceStreamEnded,
    InferenceStreamError(anyhow::Error),
    Key(KeyEvent),
    Mouse(MouseEvent),
    Repaint,
    Shutdown,
}

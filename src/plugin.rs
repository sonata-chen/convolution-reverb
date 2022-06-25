use crate::convolution::Convolution;
use std::sync::mpsc;
use std::sync::Arc;
use std::vec::Vec;

pub enum Message {
    Impulse(Arc<Vec<Vec<f32>>>),
}
pub struct AudioPlugin {
    convolution_node: Convolution,
    sample_rate: usize,
    rx: mpsc::Receiver<Message>,
}

impl AudioPlugin {
    pub fn new(sample_rate: usize) -> (mpsc::SyncSender<Message>, Self) {
        let (tx, rx) = mpsc::sync_channel(10);
        (
            tx,
            Self {
                convolution_node: Convolution::new(4096),
                sample_rate,
                rx,
            },
        )
    }
    pub fn process(&mut self, input: &[&[f32]], output: &mut [&mut [f32]]) {
        let r = self.rx.try_recv();
        if let Ok(m) = r {
            match m {
                Message::Impulse(impulse_response) => self
                    .convolution_node
                    .load_impulse_response(&impulse_response),
            }
        }
        self.convolution_node.process(input, output);
    }
}

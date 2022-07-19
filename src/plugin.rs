use crate::convolution::Convolution;
use std::sync::Arc;
use std::vec::Vec;

pub enum Message {
    Impulse(Arc<Vec<Vec<f32>>>),
}
pub struct AudioPlugin {
    convolution_node: Convolution,
    sample_rate: usize,
    buffer_size: usize,
    input_buffer: Vec<f32>,
    rx: crossbeam::channel::Receiver<Message>,
}

impl AudioPlugin {
    pub fn new() -> (crossbeam::channel::Sender<Message>, Self) {
        let (tx, rx) = crossbeam::channel::bounded(2048);
        (
            tx,
            Self {
                convolution_node: Convolution::new(4096),
                sample_rate: 0,
                buffer_size: 0,
                input_buffer: Vec::new(),
                rx,
            },
        )
    }
    pub fn prepare_to_play(&mut self, sr: usize, bs: usize) {
        self.sample_rate = sr;
        self.buffer_size = bs;
        self.input_buffer.resize(self.buffer_size, 0.0);
    }
    pub fn process<I, O>(&mut self, input: &[I], output: &mut [O])
    where
        I: AsRef<[f32]>,
        O: AsMut<[f32]>,
    {
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

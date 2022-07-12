use crate::convolution::Convolution;
use std::sync::mpsc;
use std::sync::Arc;
use std::vec::Vec;

pub enum Message {
    Impulse(Arc<Vec<Vec<f32>>>),
    Bypassed,
    Gain(f32),
}
pub struct AudioPlugin {
    convolution_node: Convolution,
    sample_rate: usize,
    buffer_size: usize,
    input_buffer: Vec<f32>,
    rx: mpsc::Receiver<Message>,

    gain: f32,
    by_passed: bool,
}

impl AudioPlugin {
    pub fn new() -> (mpsc::SyncSender<Message>, Self) {
        let (tx, rx) = mpsc::sync_channel(2048);
        (
            tx,
            Self {
                convolution_node: Convolution::new(4096),
                sample_rate: 0,
                buffer_size: 0,
                input_buffer: Vec::new(),
                rx,
                gain: 0.5,
                by_passed: false,
            },
        )
    }
    pub fn prepare_to_play(&mut self, sr: usize, bs: usize) {
        self.sample_rate = sr;
        self.buffer_size = bs;
        self.input_buffer.resize(self.buffer_size, 0.0);
    }
    pub fn process(&mut self, input: &[&[f32]], output: &mut [&mut [f32]]) {
        let r = self.rx.try_recv();
        if let Ok(m) = r {
            match m {
                Message::Impulse(impulse_response) => self
                    .convolution_node
                    .load_impulse_response(&impulse_response),
                Message::Bypassed => self.by_passed = !self.by_passed,
                Message::Gain(g) => self.gain = g,
            }
        }
        if !self.by_passed {
            self.convolution_node.process(input, output);

            for channel in output.into_iter() {
                for samples in channel.into_iter() {
                    *samples *= self.gain;
                }
            }
        } else {
            for (i_channel, o_channel) in input.into_iter().zip(output.into_iter()) {
                for (i_samples, o_samples) in i_channel.into_iter().zip(o_channel.into_iter()) {
                    *o_samples = *i_samples;
                }
            }
        }
    }
}

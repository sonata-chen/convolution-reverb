use crate::convolution::Convolution;
use crate::PlugParams;
use std::sync::Arc;
use std::vec::Vec;

pub enum Message {
    Impulse(Vec<u8>),
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
    pub fn process<I, O>(&mut self, input: &[I], output: &mut [O], params: &PlugParams)
    where
        I: AsRef<[f32]>,
        O: AsMut<[f32]>,
    {
        let r = self.rx.try_recv();
        if let Ok(m) = r {
            match m {
                Message::Impulse(impulse_response) => {
                    // let guard = params.impulse.lock().unwrap();
                    let mut reader =
                        hound::WavReader::new(std::io::BufReader::new(&impulse_response[..]))
                            .unwrap();
                    // let mut reader = hound::WavReader::open(file).unwrap();
                    println!("num of channels: {}", reader.spec().channels);
                    println!("sample rate: {}", reader.spec().sample_rate);

                    let mut iter = reader.samples::<f32>();

                    let length = iter.len();
                    println!("num of samples: {}\n\n", length);

                    let mut ir_l: Vec<f32> = Vec::with_capacity(iter.len() / 2);
                    let mut ir_r: Vec<f32> = Vec::with_capacity(iter.len() / 2);

                    for _ in 1..iter.len() {
                        if let Some(Ok(s)) = iter.next() {
                            ir_l.push(s);
                        }
                        if let Some(Ok(s)) = iter.next() {
                            ir_r.push(s);
                        }
                    }
                    let ir = Arc::new(vec![ir_l, ir_r]);
                    self.convolution_node.load_impulse_response(&ir);

                    *params.impulse.lock().unwrap() = impulse_response;
                }
            }
        }
        self.convolution_node.process(input, output);
    }
}

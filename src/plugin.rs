use crate::convolution::Convolution;
use crate::PlugParams;
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
                convolution_node: Convolution::new(1024),
                sample_rate: 0,
                buffer_size: 0,
                input_buffer: Vec::new(),
                rx,
            },
        )
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
                    let mut loader = symphonium::SymphoniumLoader::new();
                    let decoded_audio = loader.load_f32_from_source(
                        Box::new(std::io::Cursor::new(impulse_response.clone())),
                        None,
                        Some(48000),
                        symphonium::ResampleQuality::High,
                        None,
                    ).expect("Failed to read samples");

                    let channels = decoded_audio.channels();
                    let sample_rate = decoded_audio.sample_rate;
                    let frames = decoded_audio.frames();

                    eprintln!("The number of channels in the impulse response: {channels}");
                    eprintln!("Sample rate of the impulse response: {sample_rate}");
                    eprintln!(
                        "The number of samples per channel in the the impulse response: {frames}"
                    );

                    self.convolution_node.load_impulse_response(&decoded_audio.data);
                    *params.impulse.lock().unwrap() = impulse_response;
                }
            }
        }
        self.convolution_node.process(input, output);
    }
}

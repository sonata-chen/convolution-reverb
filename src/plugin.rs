use crate::convolution::Convolution;
use crate::convolution::ConvolutionEngine;
use crate::PlugParams;
use std::vec::Vec;

pub struct AudioPlugin {
    convolution_node: Convolution,
    sample_rate: usize,
    buffer_size: usize,
    input_buffer: Vec<f32>,
}

impl AudioPlugin {
    pub fn new() -> Self {
        Self {
            convolution_node: Convolution::new(1024),
            sample_rate: 0,
            buffer_size: 0,
            input_buffer: Vec::new(),
        }
    }

    pub fn swap(&mut self, engines: Vec<ConvolutionEngine>) {
        self.convolution_node.swap(engines);
    }

    pub fn process<I, O>(&mut self, input: &[I], output: &mut [O], params: &PlugParams)
    where
        I: AsRef<[f32]>,
        O: AsMut<[f32]>,
    {
        self.convolution_node.process(input, output);
    }
}

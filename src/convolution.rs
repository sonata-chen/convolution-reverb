use crate::fft::FFT;

use realfft::RealFftPlanner;
use rustfft::num_complex::Complex;
use rustfft::num_traits::Zero;

pub struct ConvolutionEngine {
    input_block_size: usize,
    fft_size: usize,
    num_segments: usize,
    num_input_segments: usize,
    buffers_impulse_segments: Vec<Vec<Complex<f32>>>,
    buffers_input_segments: Vec<Vec<Complex<f32>>>,

    buffer_input: Vec<f32>,
    buffer_c_output: Vec<Complex<f32>>,
    buffer_r_output: Vec<f32>,
    buffer_temp_output: Vec<Complex<f32>>,
    buffer_overlap: Vec<f32>,

    input_position: usize,
    current_segment: usize,

    fft: FFT,
}

impl ConvolutionEngine {
    pub fn new(samples: &[f32], max_block_size: usize) -> Self {
        let input_block_size = usize::next_power_of_two(max_block_size);

        let fft_size = if input_block_size > 128 {
            2 * input_block_size
        } else {
            4 * input_block_size
        };

        let mut real_planner = RealFftPlanner::<f32>::new();
        let num_segments = samples.len() / (fft_size - input_block_size) + 1;

        let num_input_segments = if input_block_size > 128 {
            num_segments
        } else {
            3 * num_segments
        };

        real_planner.plan_fft_forward(fft_size);
        real_planner.plan_fft_inverse(fft_size);

        // Initialize impulse segments
        let r2c = real_planner.plan_fft_forward(fft_size);
        let mut buffers_impulse_segments: Vec<Vec<Complex<f32>>> =
            vec![r2c.make_output_vec(); num_segments];

        let mut scratch = r2c.make_scratch_vec();
        let mut temp = Vec::from(samples);
        for _ in 0..(fft_size - input_block_size) {
            temp.push(0.0);
        }

        let temp_itr = temp.chunks_exact_mut(fft_size - input_block_size);
        let buf_itr = buffers_impulse_segments.iter_mut();
        let zip = temp_itr.zip(buf_itr);
        for (r, c) in zip {
            let mut input = r2c.make_input_vec();
            (&mut input[..fft_size - input_block_size]).copy_from_slice(r);
            r2c.process_with_scratch(&mut input, c, &mut scratch)
                .unwrap();
        }

        // Initialize input segments
        let buffers_input_segments = vec![r2c.make_output_vec(); num_input_segments];

        println!("input block size: {}", input_block_size);
        println!("fft size: {}", fft_size);
        println!("filter block size: {}", fft_size - input_block_size);
        println!("num of impulse segments: {}", num_segments);
        println!("num of input segments: {}", num_input_segments);
        ConvolutionEngine {
            input_block_size,
            fft_size,
            num_segments,
            num_input_segments,
            buffers_impulse_segments,
            buffers_input_segments,

            buffer_input: vec![f32::zero(); fft_size],
            buffer_c_output: vec![Complex::zero(); fft_size],
            buffer_r_output: vec![f32::zero(); fft_size],
            buffer_temp_output: vec![Complex::zero(); fft_size],
            buffer_overlap: vec![f32::zero(); fft_size],

            input_position: 0,
            current_segment: 0,

            fft: FFT::new(fft_size),
        }
    }
    pub fn process(&mut self, input: &[f32], output: &mut [f32]) {
        // assert_ne!(input.len(), output.len());
        let num_samples = input.len();

        let mut num_processed_samples = 0;

        while num_processed_samples < num_samples {
            let num_samples_to_process = usize::min(
                num_samples - num_processed_samples,
                self.input_block_size - self.input_position,
            );

            let input_frame =
                &input[num_processed_samples..num_processed_samples + num_samples_to_process];

            self.buffer_input[self.input_position..self.input_position + num_samples_to_process]
                .copy_from_slice(input_frame);

            self.fft.forward_transform(
                &self.buffer_input,
                &mut self.buffers_input_segments[self.current_segment],
            );

            if self.input_position == 0 {
                let index_step = self.num_input_segments / self.num_segments;

                self.buffer_temp_output.fill(Complex::zero());

                let mut index = self.current_segment;

                for i in 1..self.num_segments {
                    index += index_step;

                    if index >= self.num_input_segments {
                        index -= self.num_input_segments;
                    }

                    Self::convolve_and_accumulate(
                        &self.buffers_input_segments[index],
                        &self.buffers_impulse_segments[i],
                        &mut self.buffer_temp_output,
                    );
                }
            }
            self.buffer_c_output
                .copy_from_slice(&self.buffer_temp_output);

            Self::convolve_and_accumulate(
                &self.buffers_input_segments[self.current_segment],
                &self.buffers_impulse_segments[0],
                &mut self.buffer_c_output,
            );

            self.fft.inverse_transform(
                &self.buffer_c_output[0..self.buffers_impulse_segments[0].len()],
                &mut self.buffer_r_output,
            );

            // Normalization is needed. See https://github.com/HEnquist/realfft#scaling for more details.
            for i in self.buffer_r_output.iter_mut() {
                *i *= 1.0 / self.fft_size as f32;
            }

            // Add overlap
            // FloatVectorOperations::add (&output[numSamplesProcessed], &outputData[inputDataPos], &overlapData[inputDataPos], (int) numSamplesToProcess);
            Self::add(
                &mut output[num_processed_samples..],
                &self.buffer_r_output[self.input_position..],
                &self.buffer_overlap[self.input_position..],
                num_samples_to_process,
            );

            self.input_position += num_samples_to_process;

            if self.input_position == self.input_block_size {
                self.buffer_input.fill(f32::zero());
                self.input_position = 0;

                // Extra step for segSize > blockSize
                // FloatVectorOperations::add (&(outputData[blockSize]), &(overlapData[blockSize]), static_cast<int> (fftSize - 2 * blockSize));

                self.buffer_overlap[..self.fft_size - self.input_block_size]
                    .copy_from_slice(&self.buffer_r_output[self.input_block_size..]);

                self.current_segment = if self.current_segment > 0 {
                    self.current_segment - 1
                } else {
                    self.num_input_segments - 1
                }
            }

            num_processed_samples += num_samples_to_process;
        }
    }
    fn add(output: &mut [f32], v1: &[f32], v2: &[f32], samples: usize) {
        for i in 0..samples {
            output[i] = v1[i] + v2[i];
        }
    }
    fn convolve_and_accumulate(
        input: &[Complex<f32>],
        impulse: &[Complex<f32>],
        output: &mut [Complex<f32>],
    ) {
        let input = input.iter();
        let impulse = impulse.iter();
        let output = output.iter_mut();

        for ((i, im), o) in input.zip(impulse).zip(output) {
            *o += *i * *im;
        }
    }
}

pub struct Convolution {
    engines: Option<std::vec::Vec<ConvolutionEngine>>,
    num_channels: usize,
    latency: usize,
    is_stereo: bool,
}

impl Convolution {
    // pub fn new_with_impulse_data() {}
    pub fn new(fft_size: usize) -> Self {
        Self {
            engines: None,
            num_channels: 0,
            latency: fft_size,
            is_stereo: false,
        }
    }
    pub fn load_impulse_response<T>(&mut self, impulse_response: &[T])
    where
        T: AsRef<[f32]>,
    {
        let length = impulse_response.len();

        if length == 0 {
            return;
        }

        let length = if length > 2 { 2 } else { length };
        self.num_channels = length;
        self.is_stereo = length == 2;

        let mut engines = std::vec::Vec::with_capacity(self.num_channels);
        for i in 0..length {
            engines.push(ConvolutionEngine::new(
                impulse_response[i].as_ref(),
                self.latency,
            ));
        }
        self.engines = Some(engines);
    }
    pub fn process<I, O>(&mut self, input: &[I], output: &mut [O])
    where
        I: AsRef<[f32]>,
        O: AsMut<[f32]>,
    {
        if let Some(e) = &mut self.engines {
            let num_input_channels = input.len();
            let num_output_channels = output.len();
            let num_channels = usize::min(num_input_channels, num_output_channels);

            for i in 0..num_channels {
                e[i].process(input[i].as_ref(), output[i].as_mut());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ConvolutionEngine;

    #[test]
    fn rms_error() {
        const SIGNAL_LENGTH: usize = 2048;
        const BUFFER_SIZE: usize = 1024;
        // input signal
        let mut input: Vec<f32> = Vec::new();
        let ones = [1.0; SIGNAL_LENGTH];
        let tailing_zeros = [0.0; SIGNAL_LENGTH];
        input.extend_from_slice(&ones);
        input.extend_from_slice(&tailing_zeros);

        // impulse response
        let ir = [1.0; SIGNAL_LENGTH];
        let mut engine = ConvolutionEngine::new(&ir, 1024);

        // output signal
        let mut output = Vec::new();
        output.resize(SIGNAL_LENGTH * 2, 0.0);
        for i in 0..output.len() / BUFFER_SIZE {
            engine.process(
                &input[i * BUFFER_SIZE..(i + 1) * BUFFER_SIZE],
                &mut output[i * BUFFER_SIZE..(i + 1) * BUFFER_SIZE],
            );
        }

        // expected output
        let mut expected_output: Vec<f32> = Vec::new();
        expected_output.resize(SIGNAL_LENGTH * 2 - 1, 0.0);
        for i in 0..SIGNAL_LENGTH {
            expected_output[i] = i as f32 + 1.0;
        }
        for (index, s) in expected_output[SIGNAL_LENGTH..].iter_mut().enumerate() {
            *s = SIGNAL_LENGTH as f32 - 1.0 - index as f32;
        }

        // compute RMS error
        let mut sum = 0.0;
        for (o, e) in output[..output.len() - 1]
            .iter()
            .zip(expected_output.iter())
        {
            sum += (o - e).powi(2);
        }
        println!(
            "RMS error: {}",
            (sum / (SIGNAL_LENGTH as f32 * 2.0 - 1.0)).sqrt()
        );
        // eprintln!("{:?}", output);
    }
}

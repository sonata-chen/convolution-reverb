use std::sync::Arc;

use realfft::ComplexToReal;
use realfft::RealFftPlanner;
use realfft::RealToComplex;
use rustfft::num_complex::Complex;

pub struct FFT {
    r2c: Arc<dyn RealToComplex<f32>>,
    c2r: Arc<dyn ComplexToReal<f32>>,
    r_input_buffer: Vec<f32>,
    c_input_buffer: Vec<Complex<f32>>,
    r_scratch: Vec<Complex<f32>>,
    c_scratch: Vec<Complex<f32>>,
}
impl FFT {
    pub fn new(fft_size: usize) -> Self {
        let mut real_planner = RealFftPlanner::new();
        let r2c = real_planner.plan_fft_forward(fft_size);
        let c2r = real_planner.plan_fft_inverse(fft_size);
        let r_input_buffer = r2c.make_input_vec();
        let c_input_buffer = c2r.make_input_vec();
        let r_scratch = r2c.make_scratch_vec();
        let c_scratch = c2r.make_scratch_vec();

        Self {
            r2c,
            c2r,
            r_input_buffer,
            c_input_buffer,
            r_scratch,
            c_scratch,
        }
    }
    pub fn forward_transform(&mut self, input: &[f32], output: &mut [Complex<f32>]) {
        self.r_input_buffer.copy_from_slice(input);
        self.r2c
            .process_with_scratch(&mut self.r_input_buffer, output, &mut self.r_scratch)
            .unwrap();
    }
    pub fn inverse_transform(&mut self, input: &[Complex<f32>], output: &mut [f32]) {
        self.c_input_buffer.copy_from_slice(input);
        self.c2r
            .process_with_scratch(&mut self.c_input_buffer, output, &mut self.c_scratch)
            .unwrap();
    }
}

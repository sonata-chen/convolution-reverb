// mod app;
// mod convolution;
// mod fft;
// mod plugin;
// mod ui;


use ::convolution::ConvolutionReverb;
use nih_plug::prelude::*;


fn main()  {
    nih_export_standalone::<ConvolutionReverb>();
}

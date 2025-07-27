#![feature(portable_simd)]
#![feature(allocator_api)]

use nih_plug::prelude::*;
use nih_plug_vizia::{vizia::vg::rgb::bytemuck::Contiguous, ViziaState};
use std::sync::Arc;
use std::sync::Mutex;

mod allocator;
mod convolution;
mod editor;
mod fft;
mod plugin;

use convolution::ConvolutionEngine;

enum Message {
    Impulse(Vec<u8>),
    Engine(Vec<ConvolutionEngine>),
}

/// This is mostly identical to the gain example, minus some fluff, and with a GUI.
pub struct ConvolutionReverb {
    params: Arc<PlugParams>,
    sample_rate: u32,

    internal: plugin::AudioPlugin,
    tx: crossbeam::channel::Sender<Message>,
    rx: crossbeam::channel::Receiver<Message>,
    input_buffer: Vec<Vec<f32>>,

    /// Needed to normalize the peak meter's response based on the sample rate.
    peak_meter_decay_weight: f32,
    // The current data for the peak meter. This is stored as an [`Arc`] so we can share it between
    // the GUI and the audio processing parts. If you have more state to share, then it's a good
    // idea to put all of that in a struct behind a single `Arc`.
    //
    // This is stored as voltage gain.
    // peak_meter: Arc<AtomicF32>,
}

#[derive(Params)]
struct PlugParams {
    #[id = "gain"]
    pub gain: FloatParam,

    #[id = "bypassed"]
    pub bypassed: BoolParam,

    /// The editor state, saved together with the parameter state so the custom scaling can be
    /// restored.
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,

    #[persist = "impulse"]
    impulse: Arc<Mutex<Vec<u8>>>,
}

#[derive(Debug)]
pub enum BackgroundTask {
    OpenImpulse(Vec<u8>),
    ProcessImpulse(Vec<u8>, u32),
}

impl Default for ConvolutionReverb {
    fn default() -> Self {
        let plugin = plugin::AudioPlugin::new();
        let (tx, rx) = crossbeam::channel::bounded(1024);
        Self {
            params: Arc::new(PlugParams::default()),
            sample_rate: 0,

            internal: plugin,
            tx,
            rx,
            input_buffer: Vec::new(),

            peak_meter_decay_weight: 1.0,
        }
    }
}

impl Default for PlugParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new(
                "Gain",
                0.0,
                FloatRange::Linear {
                    min: -30.0,
                    max: 30.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(50.0))
            .with_step_size(0.01)
            .with_unit(" dB"),

            bypassed: BoolParam::new("Bypassed", false),
            editor_state: editor::default_state(),
            impulse: Arc::new(Mutex::new(Vec::default())),
        }
    }
}

impl Plugin for ConvolutionReverb {
    const NAME: &'static str = "CONVOLUTION";
    const VENDOR: &'static str = "example vendor";
    const URL: &'static str = "https://example.com";
    const EMAIL: &'static str = "info@example.com";

    const VERSION: &'static str = "0.0.1";

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];
    type SysExMessage = ();
    type BackgroundTask = BackgroundTask;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(
            self.params.clone(),
            self.params.editor_state.clone(),
            async_executor,
        )
    }

    fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        // TODO: How do you tie this exponential decay to an actual time span?
        self.peak_meter_decay_weight = 0.9992f32.powf(44_100.0 / buffer_config.sample_rate);
        self.sample_rate = buffer_config.sample_rate as u32;

        self.input_buffer.resize(
            audio_io_layout.main_input_channels.unwrap().into_integer() as usize,
            vec![0.0; buffer_config.max_buffer_size as usize],
        );
        {
            let ir = self.params.impulse.lock().unwrap().clone();
            if !ir.is_empty() {
                context.execute(BackgroundTask::OpenImpulse(ir));
            }
        }

        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let message = self.rx.try_recv();
        if let Ok(m) = message {
            match m {
                Message::Impulse(impulse_response) => {
                    context.execute_background(BackgroundTask::ProcessImpulse(
                        impulse_response,
                        self.sample_rate,
                    ));
                }
                Message::Engine(engines) => {
                    self.internal.swap(engines);
                }
            }
        }
        if !self.params.bypassed.value() {
            let num_channels = buffer.channels();
            let num_frames = buffer.samples();
            for c in 0..num_channels {
                for s in 0..num_frames {
                    self.input_buffer[c][s] = buffer.as_slice_immutable()[c][s];
                }
            }
            self.internal
                .process(&self.input_buffer, buffer.as_slice(), &self.params);
        }

        for channel_samples in buffer.iter_samples() {
            // let mut amplitude = 0.0;
            // let num_samples = channel_samples.len();

            let gain = self.params.gain.smoothed.next();
            for sample in channel_samples {
                *sample *= util::db_to_gain(gain);
                // amplitude += *sample;
            }
        }

        ProcessStatus::Normal
    }

    fn task_executor(&mut self) -> TaskExecutor<Self> {
        let tx = self.tx.clone();
        let impulse = self.params.impulse.clone();

        Box::new(move |task| match task {
            BackgroundTask::OpenImpulse(impulse_response) => {
                tx.send(Message::Impulse(impulse_response)).unwrap();
            }
            BackgroundTask::ProcessImpulse(impulse_response,sample_rate) => {
                let mut loader = symphonium::SymphoniumLoader::new();
                let decoded_audio = loader
                    .load_f32_from_source(
                        Box::new(std::io::Cursor::new(impulse_response.clone())),
                        None,
                        Some(sample_rate),
                        symphonium::ResampleQuality::High,
                        None,
                    )
                    .expect("Failed to read samples");

                let channels = decoded_audio.channels();
                let sample_rate = decoded_audio.sample_rate;
                let frames = decoded_audio.frames();

                eprintln!("The number of channels in the impulse response: {channels}");
                eprintln!("Sample rate of the impulse response: {sample_rate}");
                eprintln!(
                    "The number of samples per channel in the the impulse response: {frames}"
                );

                let length = decoded_audio.data.len();

                if length == 0 {
                    return;
                }

                let length = if length > 2 { 2 } else { length };

                let mut engines = std::vec::Vec::with_capacity(length);
                for i in 0..length {
                    engines.push(ConvolutionEngine::new(&decoded_audio.data[i], 1024));
                }

                *impulse.lock().unwrap() = impulse_response;
                tx.send(Message::Engine(engines)).unwrap();
            }
        })
    }
}

impl ClapPlugin for ConvolutionReverb {
    const CLAP_ID: &'static str = "com.example.convolution";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("convolution reverb");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for ConvolutionReverb {
    const VST3_CLASS_ID: [u8; 16] = *b"ConvolutionRverb";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}

nih_export_clap!(ConvolutionReverb);
nih_export_vst3!(ConvolutionReverb);

use atomic_float::AtomicF32;
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use std::sync::Arc;

mod convolution;
mod editor;
mod fft;
mod plugin;
mod ui;
// mod editor;

/// This is mostly identical to the gain example, minus some fluff, and with a GUI.
pub struct ConvolutionReverb {
    params: Arc<PlugParams>,
    editor_state: Arc<ViziaState>,

    internal: plugin::AudioPlugin,
    tx: crossbeam::channel::Sender<plugin::Message>,
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
}

impl Default for ConvolutionReverb {
    fn default() -> Self {
        let (tx, plugin) = plugin::AudioPlugin::new();
        Self {
            params: Arc::new(PlugParams::default()),
            editor_state: ViziaState::from_size(200, 150),

            internal: plugin,
            tx,
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
        }
    }
}

impl Plugin for ConvolutionReverb {
    const NAME: &'static str = "CONVOLUTION";
    const VENDOR: &'static str = "example vendor";
    const URL: &'static str = "https://example.com";
    const EMAIL: &'static str = "info@example.com";

    const VERSION: &'static str = "0.0.1";

    const DEFAULT_NUM_INPUTS: u32 = 2;
    const DEFAULT_NUM_OUTPUTS: u32 = 2;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&self) -> Option<Box<dyn Editor>> {
        editor::create(
            self.params.clone(),
            self.editor_state.clone(),
            self.tx.clone(),
        )
    }

    fn accepts_bus_config(&self, config: &BusConfig) -> bool {
        // This works with any symmetrical IO layout
        config.num_input_channels == 2
            && config.num_input_channels == config.num_output_channels
            && config.num_input_channels > 0
    }

    fn initialize(
        &mut self,
        bus_config: &BusConfig,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext,
    ) -> bool {
        // TODO: How do you tie this exponential decay to an actual time span?
        self.peak_meter_decay_weight = 0.9992f32.powf(44_100.0 / buffer_config.sample_rate);

        self.input_buffer.resize(
            bus_config.num_input_channels as usize,
            vec![0.0; dbg!{buffer_config.max_buffer_size as usize}],
        );

        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext,
    ) -> ProcessStatus {
        /*
        for channel_samples in buffer.iter_samples() {
            let mut amplitude = 0.0;
            let num_samples = channel_samples.len();

            let gain = self.params.gain.smoothed.next();
            for sample in channel_samples {
                *sample *= util::db_to_gain(gain);
                amplitude += *sample;
            }

            // To save resources, a plugin can (and probably should!) only perform expensive
            // calculations that are only displayed on the GUI while the GUI is open
            if self.editor_state.is_open() {
                amplitude = (amplitude / num_samples as f32).abs();
                let current_peak_meter = self.peak_meter.load(std::sync::atomic::Ordering::Relaxed);
                let new_peak_meter = if amplitude > current_peak_meter {
                    amplitude
                } else {
                    current_peak_meter * self.peak_meter_decay_weight
                        + amplitude * (1.0 - self.peak_meter_decay_weight)
                };

                self.peak_meter
                    .store(new_peak_meter, std::sync::atomic::Ordering::Relaxed)
            }
        }
        */
        if !self.params.bypassed.value {
            let num_channels = buffer.channels();
            let num_frames = buffer.len();
            for c in 0..num_channels {
                for s in 0..num_frames {
                    self.input_buffer[c][s] = buffer.as_slice_immutable()[c][s];
                }
            }
            self.internal.process(&self.input_buffer, buffer.as_slice());
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
    const VST3_CATEGORIES: &'static str = "Fx|Dynamics";
}

nih_export_clap!(ConvolutionReverb);
nih_export_vst3!(ConvolutionReverb);

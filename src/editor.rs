use nih_plug::context::gui::AsyncExecutor;
use nih_plug::prelude::Editor;
use std::sync::Arc;
use vizia_plug::vizia::prelude::*;
use vizia_plug::widgets::*;
use vizia_plug::{create_vizia_editor, ViziaState, ViziaTheming};

use crate::browser::{FileChooser, FileChooserModifiers};
use crate::BackgroundTask;
use crate::ConvolutionReverb;
use crate::PlugParams;

pub const NOTO_SANS: &str = "Noto Sans";

#[derive(Lens)]
struct AppData {
    params: Arc<PlugParams>,
    async_executor: AsyncExecutor<ConvolutionReverb>,
}

#[derive(Debug)]
pub enum AppEvent {
    OpenImpuseResponse(String),
}

impl Model for AppData {
    fn event(&mut self, _: &mut EventContext, event: &mut Event) {
        event.map(|app_event, _| match app_event {
            AppEvent::OpenImpuseResponse(f) => {
                let file = std::fs::read(&f).expect("Failed to read the impule!");

                self.async_executor
                    .execute_background(BackgroundTask::OpenImpulse(file))
            }
        });
    }
}

// Makes sense to also define this here, makes it a bit easier to keep track of
pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (500, 500))
}

pub(crate) fn create(
    params: Arc<PlugParams>,
    editor_state: Arc<ViziaState>,
    async_executor: AsyncExecutor<ConvolutionReverb>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(
        editor_state,
        ViziaTheming::Custom,
        move |cx, _gui_context| {
            AppData {
                params: params.clone(),
                async_executor: async_executor.clone(),
            }
            .build(cx);

            VStack::new(cx, |cx| {
                Label::new(cx, "Gain GUI")
                    .font_family(vec![FamilyOwned::Named(String::from(NOTO_SANS))])
                    .font_weight(FontWeightKeyword::Normal);

                Label::new(cx, "Gain");
                ParamSlider::new(cx, AppData::params, |params| &params.gain);

                Label::new(cx, "Mix");
                ParamSlider::new(cx, AppData::params, |params| &params.mix);

                ParamButton::new(cx, AppData::params, |params| &params.bypassed);

                FileChooser::new(cx).on_pick(|cx, f| cx.emit(AppEvent::OpenImpuseResponse(f)));
            })
            .gap(Pixels(5.0))
            .border_width(Pixels(20.0))
            .alignment(Alignment::Center);
        },
    )
}

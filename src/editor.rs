use nih_plug::prelude::Editor;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};
use std::path::PathBuf;
use std::sync::Arc;

use crate::plugin;
use crate::plugin::Message;
use crate::PlugParams;

/// VIZIA uses points instead of pixels for text
const POINT_SCALE: f32 = 0.75;

#[derive(Lens)]
struct AppData {
    params: Arc<PlugParams>,
    tx: crossbeam::channel::Sender<Message>,
}

#[derive(Debug)]
pub enum AppEvent {
    // ToggleBypassed,
    // SetGain(f32),
    LoadImpuseResponse,
}

impl Model for AppData {
    fn event(&mut self, _: &mut EventContext, event: &mut Event) {
        event.map(|app_event, _| match app_event {
            AppEvent::LoadImpuseResponse => {
                let path = PathBuf::from("data/ir.wav");
                let path = rfd::FileDialog::new().pick_file().unwrap_or(path);
                let file = std::fs::read(path).expect("Failed to read the impule!");

                self.tx.send(plugin::Message::Impulse(file)).unwrap();
            }
        });
    }
}

// Makes sense to also define this here, makes it a bit easier to keep track of
pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (600, 600))
}

pub(crate) fn create(
    params: Arc<PlugParams>,
    editor_state: Arc<ViziaState>,
    tx: crossbeam::channel::Sender<plugin::Message>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        assets::register_noto_sans_thin(cx);

        AppData {
            params: params.clone(),
            tx: tx.clone(),
        }
        .build(cx);

        VStack::new(cx, |cx| {
            Label::new(cx, "Gain GUI")
                .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                .font_weight(FontWeightKeyword::Thin)
                .font_size(40.0 * POINT_SCALE)
                .height(Pixels(150.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));

            // NOTE: VIZIA adds 1 pixel of additional height to these labels, so we'll need to
            //       compensate for that
            Label::new(cx, "Gain").bottom(Pixels(-1.0));
            ParamSlider::new(cx, AppData::params, |params| &params.gain).height(Pixels(150.0));

            Button::new(
                cx,
                |cx| cx.emit(AppEvent::LoadImpuseResponse),
                |cx| Label::new(cx, "load").width(Pixels(50.0)),
            );
        })
        .row_between(Pixels(0.0))
        .child_left(Stretch(1.0))
        .child_right(Stretch(1.0));

        ResizeHandle::new(cx);
    })
}

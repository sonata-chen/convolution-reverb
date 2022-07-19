use atomic_float::AtomicF32;
use nih_plug::prelude::{util, Editor};
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use crate::plugin;
use crate::PlugParams;

/// VIZIA uses points instead of pixels for text
const POINT_SCALE: f32 = 0.75;

const STYLE: &str = r#""#;

#[derive(Lens)]
struct AppData {
    params: Arc<PlugParams>,
    ui: crate::ui::UI,
}

#[derive(Debug)]
pub enum AppEvent {
    // ToggleBypassed,
    // SetGain(f32),
    LoadImpuseResponse,
}

impl Model for AppData {
    fn event(&mut self, _: &mut Context, event: &mut Event) {
        event.map(|app_event, _| match app_event {
            AppEvent::LoadImpuseResponse => {
                // std::thread::spawn(|| {
                //     let i = rfd::FileDialog::new().pick_file();
                //     println!("{:?}", i);
                // });
                // println!("helo");
                // self.ui.load_impulse_response("data/ir.wav");
                let file = rfd::FileDialog::new()
                    .pick_file();
                if let Some(f) = file {
                    self.ui.load_impulse_response(&f.to_string_lossy());
                }
            }
        });
    }
}

// Makes sense to also define this here, makes it a bit easier to keep track of
pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::from_size(200, 150)
}

pub(crate) fn create(
    params: Arc<PlugParams>,
    editor_state: Arc<ViziaState>,
    tx: crossbeam::channel::Sender<plugin::Message>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, move |cx, _| {
        cx.add_theme(STYLE);

        let ui = crate::ui::UI::new(tx.clone());

        AppData {
            params: params.clone(),
            ui,
        }
        .build(cx);

        ResizeHandle::new(cx);

        VStack::new(cx, |cx| {
            Label::new(cx, "Gain GUI")
                .font(assets::NOTO_SANS_THIN)
                .font_size(40.0 * POINT_SCALE)
                .height(Pixels(50.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));

            // NOTE: VIZIA adds 1 pixel of additional height to these labels, so we'll need to
            //       compensate for that
            Label::new(cx, "Gain").bottom(Pixels(-1.0));
            ParamSlider::new(cx, AppData::params, |params| &params.gain);


            Button::new(
                cx,
                |cx| cx.emit(AppEvent::LoadImpuseResponse),
                |cx| Label::new(cx, "load").width(Pixels(50.0)),
            );
        })
        .row_between(Pixels(0.0))
        .child_left(Stretch(1.0))
        .child_right(Stretch(1.0));
    })
}

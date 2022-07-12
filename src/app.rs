use std::sync::mpsc;

use vizia::prelude::*;

use crate::{plugin, ui::UI};

#[derive(Lens)]
pub struct AppData {
    ui: UI,
    bypassed: bool,
    gain: f32,
}

// Define events to mutate the data
#[derive(Debug)]
pub enum AppEvent {
    ToggleBypassed,
    SetGain(f32),
}

// Describe how the data can be mutated
impl Model for AppData {
    fn event(&mut self, _: &mut Context, event: &mut Event) {
        event.map(|app_event, _| match app_event {
            AppEvent::ToggleBypassed => {
                self.bypassed = !self.bypassed;
                self.ui
                    .send_message(|tx| tx.send(plugin::Message::Bypassed).unwrap());
            }
            AppEvent::SetGain(g) => {
                self.gain = *g;
                self.ui
                    .send_message(|tx| tx.send(plugin::Message::Gain(*g)).unwrap());
            }
        });
    }
}

impl AppData {
    fn new(ui: UI) -> Self {
        Self {
            ui,
            bypassed: false,
            gain: 0.8,
        }
    }
}

pub fn app(tx: mpsc::SyncSender<plugin::Message>) {
    let mut ui = UI::new(tx);
    ui.load_impulse_response("data/ir.wav");

    let a = Application::new(move |cx| {
        // Build the model data into the tree
        AppData::new(ui).build(cx);

        VStack::new(cx, |cx| {
            HStack::new(cx, |cx| {
                // Declare a button which emits an event
                Checkbox::new(cx, AppData::bypassed)
                    .on_toggle(|cx| cx.emit(AppEvent::ToggleBypassed));

                // Declare a label which is bound to part of the model, updating if it changes
                Label::new(cx, "Bypassed").width(Pixels(150.0));
            })
            .child_space(Stretch(1.0))
            .col_between(Pixels(6.0));

            HStack::new(cx, |cx| {
                // Declare a button which emits an event
                Slider::new(cx, AppData::gain ).on_changing(|cx, value| {
                    cx.emit(AppEvent::SetGain(value));
                });

                // Declare a label which is bound to part of the model, updating if it changes
                Label::new(cx, "Gain").width(Pixels(150.0));
            })
            .child_space(Stretch(1.0))
            .col_between(Pixels(6.0));
        });
    })
    .title("Convolution")
    .inner_size((500, 500));

    // let p = a.get_proxy();
    // p.send_event(Event::new(AppEvent::Connect(tx))).unwrap();
    a.run();
}

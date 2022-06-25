use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;

use crate::plugin;
use crate::plugin::Message;

pub struct UI {
    tx: mpsc::SyncSender<Message>,
    impulse_response: Option<Arc<Vec<Vec<f32>>>>,
}

impl UI {
    pub fn new(tx: mpsc::SyncSender<Message>) -> Self {
        Self {
            tx,
            impulse_response: None,
        }
    }
    pub fn load_impulse_response(&mut self, file_name: &str) {
        let file = PathBuf::from(file_name);

        // if !file_name.is_file() {
        //     let file = FileDialog::new()
        //         .add_filter("wav", &["wav"])
        //         .set_directory("~/")
        //         .pick_file();
        //     if let Some(f) = file {
        //         file_name = f;
        //     } else {
        //         return Ok(());
        //     }
        // }

        println!("file name: {}", file.to_string_lossy());
        let mut reader = hound::WavReader::open(file).unwrap();
        println!("num of channels: {}", reader.spec().channels);
        println!("sample rate: {}", reader.spec().sample_rate);

        let mut iter = reader.samples::<f32>();

        let length = iter.len();
        println!("num of samples: {}\n\n", length);

        let mut ir_l: Vec<f32> = Vec::with_capacity(iter.len() / 2);
        let mut ir_r: Vec<f32> = Vec::with_capacity(iter.len() / 2);

        for _ in 1..iter.len() {
            if let Some(Ok(s)) = iter.next() {
                ir_l.push(0.00004 * s);
            }
            if let Some(Ok(s)) = iter.next() {
                ir_r.push(0.00004 * s);
            }
        }
        let ir = Arc::new(vec![ir_l, ir_r]);

        self.tx
            .send(plugin::Message::Impulse(Arc::clone(&ir)))
            .unwrap();

        self.impulse_response = Some(ir);
    }
}
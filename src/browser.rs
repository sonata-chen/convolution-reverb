use glob;
use std::path::PathBuf;
use vizia_plug::vizia::prelude::*;

pub enum FileChooserEvent {
    Pick(String),
}

pub enum FileChooserModelEvent {
    Select(usize),
}

#[derive(Lens)]
struct FileChooserModel {
    dir: std::path::PathBuf,
    files: Vec<String>,
}

impl Model for FileChooserModel {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|event, _meta| match event {
            FileChooserModelEvent::Select(index) => {
                let path = self.dir.join(PathBuf::from(&self.files[*index]));
                if path.is_dir() {
                    *self = FileChooserModel::with_dir(&path);
                } else {
                    cx.emit(FileChooserEvent::Pick(
                        self.dir.join(&self.files[*index]).display().to_string(),
                    ));
                }
            }
        });
    }
}

impl Default for FileChooserModel {
    fn default() -> Self {
        Self::with_dir(&PathBuf::from(std::env::home_dir().unwrap_or("/".into())))
    }
}

impl FileChooserModel {
    fn with_dir(dir: &PathBuf) -> Self {
        let match_option = glob::MatchOptions {
            case_sensitive: true,
            require_literal_leading_dot: true,
            require_literal_separator: true,
        };

        let pattern = format!("{}/*.wav", dir.display());
        let mut files: Vec<String> = glob::glob_with(&pattern, match_option)
            .unwrap()
            .map(|f| f.unwrap().file_name().unwrap().display().to_string())
            .collect();

        let pattern = format!("{}/*/", dir.display());
        let mut dirs: Vec<String> = glob::glob_with(&pattern, match_option)
            .unwrap()
            .map(|d| d.unwrap().file_name().unwrap().display().to_string())
            .collect();

        let mut entryies = vec!["..".to_string()];
        entryies.append(&mut dirs);
        entryies.append(&mut files);

        Self {
            files: entryies,
            dir: dir.clone(),
        }
    }
}

pub trait FileChooserModifiers {
    fn on_pick<F: Fn(&mut EventContext, String) + 'static>(self, callback: F) -> Self;
}

impl<'a> FileChooserModifiers for Handle<'a, FileChooser> {
    fn on_pick<F: Fn(&mut EventContext, String) + 'static>(self, callback: F) -> Self {
        self.modify(|counter| counter.on_pick = Some(Box::new(callback)))
    }
}

pub struct FileChooser {
    on_pick: Option<Box<dyn Fn(&mut EventContext, String)>>,
}

impl View for FileChooser {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|event, _meta| match event {
            FileChooserEvent::Pick(f) => {
                if let Some(callback) = &self.on_pick {
                    (callback)(cx, f.to_string());
                }
            }
        });
    }
}

impl FileChooser {
    pub fn new(cx: &mut Context) -> Handle<Self> {
        Self { on_pick: None }.build(cx, |cx| {
            FileChooserModel::default().build(cx);
            HStack::new(cx, |cx| {
                List::new(cx, FileChooserModel::files, |cx, index, item| {
                    ListItem::new(cx, index, item, |cx, _, item| {
                        Label::new(cx, item).hoverable(false);
                    });
                })
                .selectable(Selectable::Single)
                .on_select(|cx, index| {
                    cx.emit(FileChooserModelEvent::Select(index));
                    cx.emit(ListEvent::ClearSelection);
                });
            })
            .gap(Pixels(12.0));
        })
    }
}

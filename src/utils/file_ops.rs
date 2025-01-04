use gtk::prelude::*;
use gtk::{FileDialog, Window};
use std::fs;
use std::path::PathBuf;
use glib::MainContext;
use futures_util::StreamExt;
use std::sync::{Arc, Mutex};

pub struct FileOps;

impl FileOps {
    pub fn open_file(parent_window: Option<Window>) -> Option<(PathBuf, String)> {
        let context = MainContext::default();
        let (sender, mut receiver) = futures_channel::mpsc::channel(1);
        let sender = Arc::new(Mutex::new(sender));
        
        context.spawn_local({
            let sender = sender.clone();
            async move {
                let dialog = FileDialog::new();
                dialog.set_title("Open File");
                dialog.set_modal(true);
                
                if let Ok(file) = dialog.open_future(parent_window.as_ref()).await {
                    if let Some(path) = file.path() {
                        if let Ok(content) = fs::read_to_string(&path) {
                            let _ = sender.lock().unwrap().try_send(Some((path, content)));
                            return;
                        }
                    }
                }
                let _ = sender.lock().unwrap().try_send(None);
            }
        });

        // Run the context until we get a response
        context.block_on(async {
            receiver.next().await
        }).flatten()
    }

    pub fn save_file(content: String, path: Option<PathBuf>, parent_window: Option<Window>) -> Option<PathBuf> {
        if let Some(path) = path {
            if fs::write(&path, &content).is_ok() {
                return Some(path);
            }
        }

        let context = MainContext::default();
        let (sender, mut receiver) = futures_channel::mpsc::channel(1);
        let sender = Arc::new(Mutex::new(sender));
        
        context.spawn_local({
            let sender = sender.clone();
            async move {
                let dialog = FileDialog::new();
                dialog.set_title("Save File");
                dialog.set_modal(true);
                
                if let Ok(file) = dialog.save_future(parent_window.as_ref()).await {
                    if let Some(path) = file.path() {
                        if fs::write(&path, &content).is_ok() {
                            let _ = sender.lock().unwrap().try_send(Some(path));
                            return;
                        }
                    }
                }
                let _ = sender.lock().unwrap().try_send(None);
            }
        });

        // Run the context until we get a response
        context.block_on(async {
            receiver.next().await
        }).flatten()
    }

    pub fn new_file() -> (PathBuf, String) {
        (PathBuf::from("Untitled"), String::new())
    }
} 
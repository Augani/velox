use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use velox_scene::TextureId;

use crate::decode::{self, DecodedImage};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageState {
    Pending,
    Decoding,
    Decoded,
    Uploaded(TextureId),
    Error(String),
}

enum ImageSource {
    Bytes(Vec<u8>),
    Path(PathBuf),
}

pub struct ImageHandle {
    state: Arc<Mutex<ImageState>>,
    decoded: Arc<Mutex<Option<DecodedImage>>>,
    source: Arc<Mutex<Option<ImageSource>>>,
}

impl ImageHandle {
    pub fn from_bytes(data: Vec<u8>) -> Self {
        Self {
            state: Arc::new(Mutex::new(ImageState::Pending)),
            decoded: Arc::new(Mutex::new(None)),
            source: Arc::new(Mutex::new(Some(ImageSource::Bytes(data)))),
        }
    }

    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self {
            state: Arc::new(Mutex::new(ImageState::Pending)),
            decoded: Arc::new(Mutex::new(None)),
            source: Arc::new(Mutex::new(Some(ImageSource::Path(path.into())))),
        }
    }

    pub fn state(&self) -> ImageState {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn take_decoded(&self) -> Option<DecodedImage> {
        self.decoded
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }

    pub fn set_uploaded(&self, texture_id: TextureId) {
        *self.state.lock().unwrap_or_else(|e| e.into_inner()) = ImageState::Uploaded(texture_id);
    }

    pub fn decode_sync(&self) {
        {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            match *state {
                ImageState::Pending => *state = ImageState::Decoding,
                _ => return,
            }
        }

        let source = self.source.lock().unwrap_or_else(|e| e.into_inner()).take();
        let result = match source {
            Some(ImageSource::Bytes(data)) => decode::decode_from_bytes(&data),
            Some(ImageSource::Path(path)) => decode::decode_from_path(&path),
            None => {
                *self.state.lock().unwrap_or_else(|e| e.into_inner()) =
                    ImageState::Error("no source".into());
                return;
            }
        };

        match result {
            Ok(img) => {
                *self.decoded.lock().unwrap_or_else(|e| e.into_inner()) = Some(img);
                *self.state.lock().unwrap_or_else(|e| e.into_inner()) = ImageState::Decoded;
            }
            Err(err) => {
                *self.state.lock().unwrap_or_else(|e| e.into_inner()) =
                    ImageState::Error(err.to_string());
            }
        }
    }
}

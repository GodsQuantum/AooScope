use crate::{DisplayCapabilities, DisplayDriver, DisplayError, FrameStats, validate_frame};
use asterctl_lcd::{AooScreen, AooScreenBuilder};
use image::RgbImage;
use std::path::Path;

pub struct AoostarDisplayDriver {
    screen: AooScreen,
    previous: Option<RgbImage>,
    frames_sent: u64,
}

impl AoostarDisplayDriver {
    pub fn open(device: impl AsRef<Path>) -> Result<Self, DisplayError> {
        let device = device
            .as_ref()
            .to_str()
            .ok_or_else(|| DisplayError::Backend("device path is not valid UTF-8".into()))?;
        let mut builder = AooScreenBuilder::new();
        builder.enable_cache(true);
        let screen = builder
            .open_device(device)
            .map_err(|error| DisplayError::Backend(error.to_string()))?;
        Ok(Self {
            screen,
            previous: None,
            frames_sent: 0,
        })
    }
}

impl DisplayDriver for AoostarDisplayDriver {
    fn capabilities(&self) -> DisplayCapabilities {
        DisplayCapabilities::default()
    }
    fn power_on(&mut self) -> Result<(), DisplayError> {
        self.previous = None;
        self.screen.clear_cache();
        self.screen
            .init()
            .map_err(|error| DisplayError::Backend(error.to_string()))?;
        self.screen
            .on()
            .map_err(|error| DisplayError::Backend(error.to_string()))
    }
    fn power_off(&mut self) -> Result<(), DisplayError> {
        let result = self
            .screen
            .off()
            .map_err(|error| DisplayError::Backend(error.to_string()));
        self.previous = None;
        self.screen.clear_cache();
        result
    }
    fn send_frame(&mut self, frame: &RgbImage) -> Result<FrameStats, DisplayError> {
        validate_frame(frame)?;
        self.frames_sent += 1;
        let cached = self.previous.as_ref() == Some(frame);
        if !cached {
            self.screen
                .send_image(frame)
                .map_err(|error| DisplayError::Backend(error.to_string()))?;
        }
        self.previous = Some(frame.clone());
        Ok(FrameStats {
            cached,
            bytes_sent: if cached {
                0
            } else {
                u64::from(frame.width()) * u64::from(frame.height()) * 3
            },
            frame_number: self.frames_sent,
        })
    }
}

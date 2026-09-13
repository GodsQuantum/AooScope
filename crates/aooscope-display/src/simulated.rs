use crate::{DisplayCapabilities, DisplayDriver, DisplayError, FrameStats, validate_frame};
use image::RgbImage;

pub struct SimulatedDisplayDriver {
    previous: Option<RgbImage>,
    power_events: Vec<bool>,
    frames_sent: u64,
}

impl SimulatedDisplayDriver {
    pub fn new() -> Self {
        Self {
            previous: None,
            power_events: Vec::new(),
            frames_sent: 0,
        }
    }
    pub fn power_events(&self) -> &[bool] {
        &self.power_events
    }
    pub fn frames_sent(&self) -> u64 {
        self.frames_sent
    }
}

impl Default for SimulatedDisplayDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl DisplayDriver for SimulatedDisplayDriver {
    fn capabilities(&self) -> DisplayCapabilities {
        DisplayCapabilities::default()
    }
    fn power_on(&mut self) -> Result<(), DisplayError> {
        self.power_events.push(true);
        self.previous = None;
        Ok(())
    }
    fn power_off(&mut self) -> Result<(), DisplayError> {
        self.power_events.push(false);
        self.previous = None;
        Ok(())
    }
    fn send_frame(&mut self, frame: &RgbImage) -> Result<FrameStats, DisplayError> {
        validate_frame(frame)?;
        self.frames_sent += 1;
        let cached = self.previous.as_ref() == Some(frame);
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

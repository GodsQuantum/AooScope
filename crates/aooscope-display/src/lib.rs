#![forbid(unsafe_code)]

mod aoostar;
mod simulated;

use image::RgbImage;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot, watch};

pub use aoostar::AoostarDisplayDriver;
pub use simulated::SimulatedDisplayDriver;

pub const DISPLAY_SIZE: (u32, u32) = (960, 376);
pub const DEFAULT_ANIMATION_FPS: u32 = 5;
pub const MAX_ANIMATION_FPS: u32 = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DisplayCapabilities {
    pub width: u32,
    pub height: u32,
    pub native_brightness: bool,
    pub power_control: bool,
}

impl Default for DisplayCapabilities {
    fn default() -> Self {
        Self {
            width: DISPLAY_SIZE.0,
            height: DISPLAY_SIZE.1,
            native_brightness: false,
            power_control: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameStats {
    pub cached: bool,
    pub bytes_sent: u64,
    pub frame_number: u64,
}

#[derive(Debug, Error)]
pub enum DisplayError {
    #[error("frame is {actual:?}, expected {expected:?}")]
    InvalidDimensions {
        actual: (u32, u32),
        expected: (u32, u32),
    },
    #[error("display backend: {0}")]
    Backend(String),
    #[error("frame source: {0}")]
    Source(String),
    #[error("display worker is closed")]
    WorkerClosed,
    #[error("display worker queue is full")]
    WorkerFull,
}

pub trait DisplayDriver: Send {
    fn capabilities(&self) -> DisplayCapabilities;
    fn power_on(&mut self) -> Result<(), DisplayError>;
    fn power_off(&mut self) -> Result<(), DisplayError>;
    fn send_frame(&mut self, frame: &RgbImage) -> Result<FrameStats, DisplayError>;
}

#[derive(Clone)]
pub struct DisplayWorker {
    tx: tokio::sync::mpsc::Sender<Command>,
    power_on: Arc<AtomicBool>,
}

enum Command {
    PowerOn(oneshot::Sender<Result<(), DisplayError>>),
    PowerOff(oneshot::Sender<Result<(), DisplayError>>),
    Frame(RgbImage, oneshot::Sender<Result<FrameStats, DisplayError>>),
}

impl DisplayWorker {
    pub fn disabled() -> Self {
        let (tx, rx) = mpsc::channel(1);
        drop(rx);
        Self {
            tx,
            power_on: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn spawn<D: DisplayDriver + 'static>(driver: D, capacity: usize) -> Self {
        let (tx, mut rx) = mpsc::channel(capacity.max(1));
        let power_on = Arc::new(AtomicBool::new(false));
        let state = power_on.clone();
        tokio::task::spawn_blocking(move || {
            let mut driver = driver;
            while let Some(command) = rx.blocking_recv() {
                match command {
                    Command::PowerOn(reply) => {
                        let result = driver.power_on();
                        if result.is_ok() {
                            state.store(true, Ordering::Release);
                        }
                        let _ = reply.send(result);
                    }
                    Command::PowerOff(reply) => {
                        let result = driver.power_off();
                        if result.is_ok() {
                            state.store(false, Ordering::Release);
                        }
                        let _ = reply.send(result);
                    }
                    Command::Frame(frame, reply) => {
                        let _ = reply.send(driver.send_frame(&frame));
                    }
                }
            }
        });
        Self { tx, power_on }
    }

    pub fn power_state(&self) -> bool {
        self.power_on.load(Ordering::Acquire)
    }

    pub async fn power_on(&self) -> Result<(), DisplayError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(Command::PowerOn(reply_tx))
            .await
            .map_err(|_| DisplayError::WorkerClosed)?;
        reply_rx.await.map_err(|_| DisplayError::WorkerClosed)?
    }

    pub async fn power_off(&self) -> Result<(), DisplayError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(Command::PowerOff(reply_tx))
            .await
            .map_err(|_| DisplayError::WorkerClosed)?;
        reply_rx.await.map_err(|_| DisplayError::WorkerClosed)?
    }

    pub async fn send_frame(&self, frame: RgbImage) -> Result<FrameStats, DisplayError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .try_send(Command::Frame(frame, reply_tx))
            .map_err(|error| match error {
                tokio::sync::mpsc::error::TrySendError::Full(_) => DisplayError::WorkerFull,
                tokio::sync::mpsc::error::TrySendError::Closed(_) => DisplayError::WorkerClosed,
            })?;
        reply_rx.await.map_err(|_| DisplayError::WorkerClosed)?
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotedRevision {
    pub id: String,
}

impl PromotedRevision {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AnimationSchedule {
    pub fps: u32,
    pub interval: Duration,
}

impl AnimationSchedule {
    pub fn new(fps: u32) -> Self {
        let fps = fps.clamp(1, MAX_ANIMATION_FPS);
        Self {
            fps,
            interval: Duration::from_nanos(1_000_000_000 / u64::from(fps)),
        }
    }
}

#[derive(Default)]
pub struct DisplayScheduler {
    current: Option<PromotedRevision>,
}

impl DisplayScheduler {
    pub fn promote(&mut self, revision: PromotedRevision) {
        self.current = Some(revision);
    }
    pub fn current_revision(&self) -> Option<&PromotedRevision> {
        self.current.as_ref()
    }
    pub fn refresh_interval(&self, animation: bool) -> Duration {
        if animation {
            AnimationSchedule::new(DEFAULT_ANIMATION_FPS).interval
        } else {
            Duration::from_secs(1)
        }
    }
    pub fn animation_schedule(&self, requested_fps: u32) -> AnimationSchedule {
        AnimationSchedule::new(if requested_fps == 0 {
            DEFAULT_ANIMATION_FPS
        } else {
            requested_fps
        })
    }

    pub async fn run<S: FrameSource>(
        mut self,
        worker: DisplayWorker,
        mut source: S,
        mut promoted: watch::Receiver<Option<PromotedRevision>>,
        animation: bool,
        requested_fps: u32,
    ) -> Result<(), DisplayError> {
        self.current = promoted.borrow().clone();
        let mut animated = self
            .current
            .as_ref()
            .and_then(|revision| source.animation_fps(revision).ok().flatten())
            .or_else(|| animation.then_some(requested_fps));
        let mut ticker = tokio::time::interval(
            animated
                .map(|fps| self.animation_schedule(fps).interval)
                .unwrap_or(Duration::from_secs(1)),
        );
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                changed = promoted.changed() => {
                    changed.map_err(|_| DisplayError::WorkerClosed)?;
                    self.current = promoted.borrow().clone();
                    animated = self.current.as_ref()
                        .and_then(|revision| source.animation_fps(revision).ok().flatten())
                        .or_else(|| animation.then_some(requested_fps));
                    ticker = tokio::time::interval(animated
                        .map(|fps| self.animation_schedule(fps).interval)
                        .unwrap_or(Duration::from_secs(1)));
                    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                }
                _ = ticker.tick() => {
                    if let Some(revision) = self.current.as_ref() {
                        let result = match source.frame(revision, animated.is_some()) {
                            Ok(frame) => {
                                validate_frame(&frame).map(|()| frame)
                            }
                            Err(error) => Err(error),
                        };
                        let result = match result {
                            Ok(frame) => worker.send_frame(frame).await,
                            Err(error) => Err(error),
                        };
                        if let Err(error) = result {
                            if matches!(error, DisplayError::WorkerClosed) {
                                return Err(error);
                            }
                            tracing::warn!(%error, "display tick skipped");
                        }
                    }
                }
            }
        }
    }
}

pub trait FrameSource: Send + 'static {
    fn animation_fps(&mut self, _revision: &PromotedRevision) -> Result<Option<u32>, DisplayError> {
        Ok(None)
    }

    fn frame(
        &mut self,
        revision: &PromotedRevision,
        animation: bool,
    ) -> Result<RgbImage, DisplayError>;
}

fn validate_frame(frame: &RgbImage) -> Result<(), DisplayError> {
    if frame.dimensions() != DISPLAY_SIZE {
        return Err(DisplayError::InvalidDimensions {
            actual: frame.dimensions(),
            expected: DISPLAY_SIZE,
        });
    }
    Ok(())
}

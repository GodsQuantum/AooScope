use aooscope_display::{
    AnimationSchedule, DISPLAY_SIZE, DisplayCapabilities, DisplayDriver, DisplayError,
    DisplayScheduler, FrameStats, PromotedRevision, SimulatedDisplayDriver,
};
use image::RgbImage;
use std::time::Duration;
use tokio::sync::watch;

#[test]
fn simulated_driver_records_power_and_differential_frames() -> Result<(), DisplayError> {
    let mut driver = SimulatedDisplayDriver::new();
    assert_eq!(driver.capabilities(), DisplayCapabilities::default());
    assert!(!driver.capabilities().native_brightness);

    driver.power_on()?;
    let frame = RgbImage::new(DISPLAY_SIZE.0, DISPLAY_SIZE.1);
    let first = driver.send_frame(&frame)?;
    let second = driver.send_frame(&frame)?;
    driver.power_off()?;
    driver.power_on()?;
    let after_cycle = driver.send_frame(&frame)?;

    assert_eq!(
        first,
        FrameStats {
            cached: false,
            bytes_sent: u64::from(DISPLAY_SIZE.0 * DISPLAY_SIZE.1 * 3),
            frame_number: 1
        }
    );
    assert_eq!(
        second,
        FrameStats {
            cached: true,
            bytes_sent: 0,
            frame_number: 2
        }
    );
    assert_eq!(driver.power_events(), &[true, false, true]);
    assert!(!after_cycle.cached);
    assert_eq!(driver.frames_sent(), 3);
    Ok(())
}

#[test]
fn scheduler_promotes_only_revisions_and_clamps_animation_fps() {
    let mut scheduler = DisplayScheduler::default();
    assert_eq!(scheduler.current_revision(), None);
    scheduler.promote(PromotedRevision::new("r1"));
    assert_eq!(scheduler.current_revision().unwrap().id, "r1");
    assert_eq!(scheduler.refresh_interval(false), Duration::from_secs(1));
    assert_eq!(scheduler.animation_schedule(99), AnimationSchedule::new(8));
    assert_eq!(scheduler.animation_schedule(0), AnimationSchedule::new(5));
}

#[tokio::test]
async fn worker_has_bounded_backpressure_and_serializes_replies() {
    let worker = aooscope_display::DisplayWorker::spawn(SlowDriver, 1);
    let frame = RgbImage::new(DISPLAY_SIZE.0, DISPLAY_SIZE.1);
    let first = worker.send_frame(frame.clone());
    let second = worker.send_frame(frame.clone());
    let third = worker.send_frame(frame);
    let results = tokio::join!(first, second, third);
    assert!(results.0.is_ok());
    assert!(
        matches!(results.1, Err(DisplayError::WorkerFull))
            || matches!(results.2, Err(DisplayError::WorkerFull))
    );
}

#[tokio::test(start_paused = true)]
async fn scheduler_waits_for_promotion_and_clamps_runtime_fps() {
    let worker = aooscope_display::DisplayWorker::spawn(SimulatedDisplayDriver::new(), 4);
    let (promoted_tx, promoted_rx) = watch::channel(None);
    let source = CountingSource::default();
    let count = source.count.clone();
    let task = tokio::spawn(aooscope_display::DisplayScheduler::default().run(
        worker,
        source,
        promoted_rx,
        true,
        99,
    ));
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(1)).await;
    tokio::task::yield_now().await;
    assert_eq!(*count.lock().unwrap(), 0);
    promoted_tx.send(Some(PromotedRevision::new("r1"))).unwrap();
    tokio::task::yield_now().await;
    for _ in 0..6 {
        tokio::time::advance(Duration::from_millis(125)).await;
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }
    }
    tokio::task::spawn_blocking(|| {}).await.unwrap();
    assert!((1..=5).contains(&*count.lock().unwrap()));
    task.abort();
}

#[tokio::test(start_paused = true)]
async fn scheduler_skips_transient_source_and_driver_errors() {
    let (success_tx, success_rx) = std::sync::mpsc::channel();
    let driver = FlakyDriver {
        success_tx: Some(success_tx),
        ..Default::default()
    };
    let frames = driver.frames.clone();
    let worker = aooscope_display::DisplayWorker::spawn(driver, 4);
    let (_promoted_tx, promoted_rx) = watch::channel(Some(PromotedRevision::new("r1")));
    let task = tokio::spawn(aooscope_display::DisplayScheduler::default().run(
        worker,
        FlakySource::default(),
        promoted_rx,
        true,
        8,
    ));

    tokio::task::yield_now().await;
    for _ in 0..6 {
        tokio::time::advance(Duration::from_millis(125)).await;
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }
    }
    tokio::task::spawn_blocking(move || success_rx.recv_timeout(Duration::from_secs(1)))
        .await
        .unwrap()
        .expect("scheduler should recover and submit a later frame");
    assert!(!task.is_finished());
    assert!(*frames.lock().unwrap() >= 1);
    task.abort();
}

#[derive(Default)]
struct CountingSource {
    count: std::sync::Arc<std::sync::Mutex<u32>>,
}

impl aooscope_display::FrameSource for CountingSource {
    fn frame(&mut self, _: &PromotedRevision, _: bool) -> Result<RgbImage, DisplayError> {
        *self.count.lock().unwrap() += 1;
        Ok(RgbImage::new(DISPLAY_SIZE.0, DISPLAY_SIZE.1))
    }
}

struct SlowDriver;

#[derive(Default)]
struct FlakySource {
    calls: u8,
}

impl aooscope_display::FrameSource for FlakySource {
    fn frame(&mut self, _: &PromotedRevision, _: bool) -> Result<RgbImage, DisplayError> {
        self.calls += 1;
        if self.calls == 1 {
            return Err(DisplayError::Source("transient".into()));
        }
        Ok(RgbImage::new(DISPLAY_SIZE.0, DISPLAY_SIZE.1))
    }
}

#[derive(Default)]
struct FlakyDriver {
    calls: std::sync::Arc<std::sync::Mutex<u8>>,
    frames: std::sync::Arc<std::sync::Mutex<u8>>,
    success_tx: Option<std::sync::mpsc::Sender<()>>,
}

impl DisplayDriver for FlakyDriver {
    fn capabilities(&self) -> DisplayCapabilities {
        DisplayCapabilities::default()
    }
    fn power_on(&mut self) -> Result<(), DisplayError> {
        Ok(())
    }
    fn power_off(&mut self) -> Result<(), DisplayError> {
        Ok(())
    }
    fn send_frame(&mut self, _: &RgbImage) -> Result<FrameStats, DisplayError> {
        let mut calls = self.calls.lock().unwrap();
        *calls += 1;
        if *calls == 1 {
            return Err(DisplayError::Backend("transient".into()));
        }
        *self.frames.lock().unwrap() += 1;
        if let Some(success_tx) = &self.success_tx {
            let _ = success_tx.send(());
        }
        Ok(FrameStats {
            cached: false,
            bytes_sent: 1,
            frame_number: u64::from(*calls),
        })
    }
}

impl DisplayDriver for SlowDriver {
    fn capabilities(&self) -> DisplayCapabilities {
        DisplayCapabilities::default()
    }
    fn power_on(&mut self) -> Result<(), DisplayError> {
        Ok(())
    }
    fn power_off(&mut self) -> Result<(), DisplayError> {
        Ok(())
    }
    fn send_frame(&mut self, _: &RgbImage) -> Result<FrameStats, DisplayError> {
        std::thread::sleep(Duration::from_millis(25));
        Ok(FrameStats {
            cached: false,
            bytes_sent: 1,
            frame_number: 1,
        })
    }
}

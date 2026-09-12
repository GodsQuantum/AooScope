use crate::state::AppState;
use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use futures_util::{Stream, stream};
use std::{convert::Infallible, time::Duration};

pub async fn get_events(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let receiver = state.subscribe_status();
    let stream = stream::unfold(receiver, |mut receiver| async move {
        if receiver.changed().await.is_err() {
            return None;
        }
        let status = receiver.borrow_and_update().clone();
        let data = serde_json::to_string(&status).expect("StatusDto serialization");
        Some((Ok(Event::default().event("status").data(data)), receiver))
    });
    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keepalive"),
    )
}

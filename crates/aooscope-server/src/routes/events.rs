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
    let status = state.subscribe_status();
    let media = state.subscribe_media();
    let stream = stream::unfold((status, media), |(mut status, mut media)| async move {
        tokio::select! {
            changed = status.changed() => {
                if changed.is_err() { return None; }
                let data = serde_json::to_string(&*status.borrow_and_update()).expect("StatusDto serialization");
                Some((Ok(Event::default().event("status").data(data)), (status, media)))
            }
            changed = media.changed() => {
                if changed.is_err() { return None; }
                let data = serde_json::to_string(&*media.borrow_and_update()).expect("MediaDisplayEvent serialization");
                Some((Ok(Event::default().event("media").data(data)), (status, media)))
            }
        }
    });
    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keepalive"),
    )
}

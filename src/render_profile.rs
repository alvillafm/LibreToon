//! Opt-in frame instrumentation. No clocks or allocations on the normal path.
use std::{cell::RefCell, collections::BTreeMap, time::Instant};
thread_local! {static FRAME:RefCell<Option<(Instant,BTreeMap<&'static str,f64>)>>=const{RefCell::new(None)};}
pub(crate) struct Frame(bool);
pub(crate) fn frame() -> Frame {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let enabled = *ENABLED.get_or_init(|| std::env::var_os("LIBRETOON_RENDER_PROFILE").is_some());
    if enabled {
        FRAME.with(|frame| *frame.borrow_mut() = Some((Instant::now(), BTreeMap::new())));
    }
    Frame(enabled)
}
impl Drop for Frame {
    fn drop(&mut self) {
        if self.0 {
            FRAME.with(|frame| {
                if let Some((start, times)) = frame.borrow_mut().take() {
                    eprintln!(
                        "render profile: total {:.2} ms; {times:?}",
                        start.elapsed().as_secs_f64() * 1000.
                    );
                }
            });
        }
    }
}
pub(crate) struct Span(&'static str, Option<Instant>);
pub(crate) fn span(name: &'static str) -> Span {
    Span(
        name,
        FRAME
            .with(|frame| frame.borrow().is_some())
            .then(Instant::now),
    )
}
impl Drop for Span {
    fn drop(&mut self) {
        if let Some(start) = self.1 {
            FRAME.with(|frame| {
                if let Some((_, times)) = frame.borrow_mut().as_mut() {
                    *times.entry(self.0).or_default() += start.elapsed().as_secs_f64() * 1000.;
                }
            });
        }
    }
}

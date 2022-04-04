use std::sync::Once;

pub fn initialize() {
    static ONCE: Once = Once::new();
    ONCE.call_once(move || {
        tracing_subscriber::fmt::init();
    });
}

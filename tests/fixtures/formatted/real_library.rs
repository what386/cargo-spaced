// Builds a client.
// The client owns its transport.
use std::time::Duration;

mod support {
    const BUFFER_SIZE: usize = 4096;

    mod io;

    fn default_timeout() -> Duration {
        Duration::from_secs(5)
    }
}

pub struct Client {
    timeout: Duration,
}

impl Client {
    pub fn new() -> Self {
        let timeout = default_timeout()
            .checked_add(Duration::from_secs(1))
            .unwrap();

        if timeout.is_zero() {
            panic!("invalid timeout");
        }

        Self { timeout }
    }
}

fn finish() {}

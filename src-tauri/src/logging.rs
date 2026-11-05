//! File logging with secret redaction.
//!
//! Logs rotate daily in the app log directory. Every line passes through
//! [`l8_core::redact::redact_secrets`] before it is written, so a token that slips into a log
//! message (an error body, a URL) never reaches the disk.

use std::io::{self, Write};
use std::path::Path;

use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::MakeWriter;

const LOG_FILE_PREFIX: &str = "l8-play.log";
const DEFAULT_FILTER: &str = "info,l8_core=debug,l8_play=debug";

/// Keeps the background log writer alive; stored in Tauri state for the app's lifetime.
pub struct LogGuard(#[allow(dead_code)] WorkerGuard);

pub fn init(log_dir: &Path) -> io::Result<LogGuard> {
    std::fs::create_dir_all(log_dir)?;
    let appender = tracing_appender::rolling::daily(log_dir, LOG_FILE_PREFIX);
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false)
        .with_writer(RedactingMakeWriter(writer))
        .finish();
    // A global subscriber can only be set once; a second call (tests, reloads) keeps the first.
    let _ = tracing::subscriber::set_global_default(subscriber);
    Ok(LogGuard(guard))
}

#[derive(Clone)]
struct RedactingMakeWriter(NonBlocking);

impl<'a> MakeWriter<'a> for RedactingMakeWriter {
    type Writer = RedactingWriter;

    fn make_writer(&'a self) -> Self::Writer {
        RedactingWriter(self.0.clone())
    }
}

struct RedactingWriter(NonBlocking);

impl Write for RedactingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let text = String::from_utf8_lossy(buf);
        self.0.write_all(l8_core::redact::redact_secrets(&text).as_bytes())?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

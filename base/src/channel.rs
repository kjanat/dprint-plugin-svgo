use std::sync::Arc;
use std::time::Duration;

use deno_core::anyhow::Error;
use deno_core::parking_lot::Mutex;
use dprint_core::async_runtime::async_trait;
use dprint_core::plugins::FormatRequest;
use dprint_core::plugins::FormatResult;
use sysinfo::MemoryRefreshKind;
use sysinfo::System;
use tokio::sync::oneshot;

use crate::util::create_tokio_runtime;

/// Safety margin multiplier for memory checks.
/// 2.2x provides buffer for concurrent plugin instances to avoid going over memory limits.
const MEMORY_SAFETY_MARGIN: f64 = 2.2;

/// Channel capacity for format requests. Provides backpressure under heavy load.
const CHANNEL_CAPACITY: usize = 100;

#[async_trait(?Send)]
pub trait Formatter<TConfiguration> {
  async fn format_text(
    &mut self,
    request: FormatRequest<TConfiguration>,
  ) -> Result<Option<Vec<u8>>, Error>;
}

pub type CreateFormatterCb<TConfiguration> =
  dyn Fn() -> Box<dyn Formatter<TConfiguration>> + Send + Sync;

pub struct CreateChannelOptions<TConfiguration> {
  /// The amount of memory that a single isolate might use. You can approximate
  /// this value out by launching the plugin with `DPRINT_MAX_THREADS=1` and seeing
  /// how much memory is used when formatting some large files.
  ///
  /// This provides some protection against using too much memory on the system,
  /// but is not perfect. It is better than nothing.
  pub avg_isolate_memory_usage: usize,
  pub create_formatter_cb: Arc<CreateFormatterCb<TConfiguration>>,
}

type Request<TConfiguration> = (FormatRequest<TConfiguration>, oneshot::Sender<FormatResult>);

struct Stats {
  pending_runtimes: usize,
  total_runtimes: usize,
}

pub struct Channel<TConfiguration: Send + Sync + 'static> {
  stats: Arc<Mutex<Stats>>,
  sys: Mutex<System>,
  sender: async_channel::Sender<Request<TConfiguration>>,
  receiver: async_channel::Receiver<Request<TConfiguration>>,
  options: CreateChannelOptions<TConfiguration>,
}

impl<TConfiguration: Send + Sync + 'static> Channel<TConfiguration> {
  #[must_use]
  pub fn new(options: CreateChannelOptions<TConfiguration>) -> Self {
    let (sender, receiver) = async_channel::bounded(CHANNEL_CAPACITY);
    Self {
      stats: Arc::new(Mutex::new(Stats {
        pending_runtimes: 0,
        total_runtimes: 0,
      })),
      sys: Mutex::new(System::new()),
      sender,
      receiver,
      options,
    }
  }

  /// Formats text using a pooled JS runtime.
  ///
  /// # Errors
  ///
  /// Returns an error if the format request fails or channel communication fails.
  pub async fn format(&self, request: FormatRequest<TConfiguration>) -> FormatResult {
    let (send, recv) = oneshot::channel::<FormatResult>();
    let should_create_runtime = {
      let mut stats = self.stats.lock();
      if stats.pending_runtimes == 0 && (stats.total_runtimes == 0 || self.has_memory_available()) {
        stats.total_runtimes += 1;
        // Don't increment pending_runtimes here - the runtime will do it when ready
        true
      } else {
        false
      }
    };

    if should_create_runtime {
      self.create_js_runtime();
    }

    self.sender.send((request, send)).await?;

    recv.await?
  }

  fn has_memory_available(&self) -> bool {
    // Only allow creating another instance if the amount of available
    // memory on the system is greater than a comfortable amount
    let mut sys = self.sys.lock();
    sys.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
    let available_memory = sys.available_memory();
    // Use safety margin to prevent concurrent plugins from exceeding memory limits
    #[expect(
      clippy::cast_precision_loss,
      clippy::cast_possible_truncation,
      clippy::cast_sign_loss
    )]
    let threshold = (self.options.avg_isolate_memory_usage as f64 * MEMORY_SAFETY_MARGIN) as u64;
    available_memory > threshold
  }

  fn create_js_runtime(&self) {
    let stats = self.stats.clone();
    let receiver = self.receiver.clone();
    let create_formatter_cb = self.options.create_formatter_cb.clone();
    std::thread::spawn(move || {
      let tokio_runtime = create_tokio_runtime();
      tokio_runtime.block_on(async move {
        let mut formatter = (create_formatter_cb)();

        // Mark this runtime as pending now that it's ready to receive work
        stats.lock().pending_runtimes += 1;

        loop {
          // Use biased selection to prioritize processing requests over idle timeout
          tokio::select! {
            biased;

            request = receiver.recv() => {
              let (request, response) = match request {
                Ok(result) => result,
                Err(_) => {
                  // Channel closed, clean up and exit
                  let mut stats = stats.lock();
                  stats.total_runtimes -= 1;
                  stats.pending_runtimes -= 1;
                  return;
                }
              };
              let result = formatter.format_text(request).await;
              let _ = response.send(result);
            }
            // Automatically shut down after idle timeout to save memory in editor scenarios
            () = tokio::time::sleep(Duration::from_secs(30)) => {
              // Only shut down if we're not the last runtime
              let mut stats = stats.lock();
              if stats.total_runtimes > 1 {
                stats.total_runtimes -= 1;
                stats.pending_runtimes -= 1;
                return;
              }
              // We're the last runtime, keep it alive
            }
          }
        }
      });
    });
  }
}

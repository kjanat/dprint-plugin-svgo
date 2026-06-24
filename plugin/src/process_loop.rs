use std::collections::HashMap;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Write;
use std::rc::Rc;
use std::sync::Arc;

use deno_core::anyhow::Context;
use deno_core::anyhow::Result;
use deno_core::anyhow::anyhow;
use deno_core::anyhow::bail;
use deno_core::futures::StreamExt;
use deno_core::futures::stream::FuturesUnordered;
use dprint_core::async_runtime::FutureExt;
use dprint_core::communication::Message;
use dprint_core::communication::MessageReader;
use dprint_core::communication::MessageWriter;
use dprint_core::configuration::ConfigKeyMap;
use dprint_core::configuration::ConfigurationDiagnostic;
use dprint_core::configuration::GlobalConfiguration;
use dprint_core::plugins::AsyncPluginHandler;
use dprint_core::plugins::FileMatchingInfo;
use dprint_core::plugins::FormatRequest;
use dprint_core::plugins::HostFormatRequest;
use serde::Serialize;
use tokio_util::sync::CancellationToken;

use crate::process_messages::CheckConfigUpdatesMessageBody;
use crate::process_messages::CheckConfigUpdatesResponseBody;
use crate::process_messages::MessageBody;
use crate::process_messages::PLUGIN_SCHEMA_VERSION;
use crate::process_messages::ProcessPluginMessage;
use crate::process_messages::ResponseBody;

struct StoredConfig<TConfiguration: Serialize + Clone> {
  config: Arc<TConfiguration>,
  diagnostics: Rc<Vec<ConfigurationDiagnostic>>,
  file_matching: FileMatchingInfo,
  config_map: ConfigKeyMap,
  global_config: GlobalConfiguration,
}

/// Handles process-plugin stdio messages without the async stdout writer thread.
///
/// Reads from stdin and writes to stdout, wiring the host's cooperative
/// cancellation protocol into the formatter (see [`run_message_loop`]).
pub async fn handle_process_stdio_messages_sync<THandler: AsyncPluginHandler>(
  handler: THandler,
) -> Result<()> {
  dprint_core::plugins::process::setup_exit_process_panic_hook();

  let stdin_reader = MessageReader::new(std::io::stdin());
  let stdout_writer = MessageWriter::new(std::io::stdout());
  run_message_loop(handler, stdin_reader, stdout_writer).await
}

/// Drives the process-plugin message loop over the given reader/writer.
///
/// Unlike a simple request/response loop, `Format` requests are run
/// concurrently so the loop can keep servicing messages — most importantly
/// `CancelFormat` — while a format is in flight. Each in-flight format owns a
/// [`CancellationToken`] keyed by its request message id; `CancelFormat`
/// cancels and removes that token. When a format finishes, its response is only
/// sent if the token is still present (i.e. the host did not cancel it), which
/// matches the dprint protocol: a cancelled format gets no reply.
///
/// Concurrency stays on a single thread (`FuturesUnordered`, not `spawn`)
/// because the handler future is `?Send`. The blocking stdin read is moved onto
/// a dedicated thread that forwards parsed messages over a channel.
async fn run_message_loop<THandler, TRead, TWrite>(
  handler: THandler,
  mut stdin_reader: MessageReader<TRead>,
  mut stdout_writer: MessageWriter<TWrite>,
) -> Result<()>
where
  THandler: AsyncPluginHandler,
  TRead: Read + Unpin + Send + 'static,
  TWrite: Write + Unpin,
{
  schema_establishment_phase(&mut stdin_reader, &mut stdout_writer)
    .context("Failed establishing schema.")?;

  let message_rx = spawn_message_reader(stdin_reader);
  drive_message_loop(handler, stdout_writer, message_rx).await
}

fn spawn_message_reader<TRead>(
  mut stdin_reader: MessageReader<TRead>,
) -> tokio::sync::mpsc::UnboundedReceiver<std::io::Result<ProcessPluginMessage>>
where
  TRead: Read + Unpin + Send + 'static,
{
  let (message_tx, message_rx) =
    tokio::sync::mpsc::unbounded_channel::<std::io::Result<ProcessPluginMessage>>();
  std::thread::spawn(move || {
    loop {
      let message = ProcessPluginMessage::read(&mut stdin_reader);
      let stop = message.is_err();
      // If the receiver is gone the loop is shutting down. Stop after a read
      // error too (EOF/broken pipe or a genuine protocol failure).
      if message_tx.send(message).is_err() || stop {
        break;
      }
    }
  });

  message_rx
}

async fn drive_message_loop<THandler, TWrite>(
  handler: THandler,
  mut stdout_writer: MessageWriter<TWrite>,
  mut message_rx: tokio::sync::mpsc::UnboundedReceiver<std::io::Result<ProcessPluginMessage>>,
) -> Result<()>
where
  THandler: AsyncPluginHandler,
  TWrite: Write + Unpin,
{
  let mut next_message_id = 1_u32;
  let mut configs = HashMap::<u32, Rc<StoredConfig<THandler::Configuration>>>::new();
  // Cancellation tokens for in-flight formats, keyed by request message id.
  let mut format_tokens = HashMap::<u32, CancellationToken>::new();
  // Concurrently running formats; each resolves to (request message id, result).
  let mut in_flight = FuturesUnordered::new();
  let mut input_closed = false;

  loop {
    if input_closed && in_flight.is_empty() {
      return Ok(());
    }

    tokio::select! {
      // Prefer queued input over starting pending formats. This lets a queued
      // `CancelFormat` mark the token before the formatter first observes it.
      biased;

      message = message_rx.recv(), if !input_closed => {
        let message = match message {
          None => {
            input_closed = true;
            continue;
          }
          Some(Ok(message)) => message,
          Some(Err(err))
            if matches!(err.kind(), ErrorKind::UnexpectedEof | ErrorKind::BrokenPipe) =>
          {
            input_closed = true;
            continue;
          }
          Some(Err(err)) => return Err(err.into()),
        };

        match message.body {
          MessageBody::Close => {
            send_response_body(
              &mut stdout_writer,
              &mut next_message_id,
              MessageBody::Success(message.id),
            )?;
            return Ok(());
          }
          MessageBody::IsAlive => {
            send_response_body(
              &mut stdout_writer,
              &mut next_message_id,
              MessageBody::Success(message.id),
            )?;
          }
          MessageBody::GetPluginInfo => {
            let data = serde_json::to_vec(&handler.plugin_info())?;
            send_response_body(
              &mut stdout_writer,
              &mut next_message_id,
              MessageBody::DataResponse(ResponseBody {
                message_id: message.id,
                data,
              }),
            )?;
          }
          MessageBody::GetLicenseText => {
            send_response_body(
              &mut stdout_writer,
              &mut next_message_id,
              MessageBody::DataResponse(ResponseBody {
                message_id: message.id,
                data: handler.license_text().into_bytes(),
              }),
            )?;
          }
          MessageBody::RegisterConfig(body) => {
            let global_config: GlobalConfiguration = serde_json::from_slice(&body.global_config)?;
            let config_map: ConfigKeyMap = serde_json::from_slice(&body.plugin_config)?;
            let result = handler
              .resolve_config(config_map.clone(), global_config.clone())
              .await;
            configs.insert(
              body.config_id.as_raw(),
              Rc::new(StoredConfig {
                config: Arc::new(result.config),
                diagnostics: Rc::new(result.diagnostics),
                file_matching: result.file_matching,
                config_map,
                global_config,
              }),
            );
            send_response_body(
              &mut stdout_writer,
              &mut next_message_id,
              MessageBody::Success(message.id),
            )?;
          }
          MessageBody::ReleaseConfig(config_id) => {
            configs.remove(&config_id.as_raw());
            send_response_body(
              &mut stdout_writer,
              &mut next_message_id,
              MessageBody::Success(message.id),
            )?;
          }
          MessageBody::GetConfigDiagnostics(config_id) => {
            let diagnostics = configs
              .get(&config_id.as_raw())
              .map(|config| config.diagnostics.clone())
              .unwrap_or_else(|| Rc::new(Vec::new()));
            let data = serde_json::to_vec(&*diagnostics)?;
            send_response_body(
              &mut stdout_writer,
              &mut next_message_id,
              MessageBody::DataResponse(ResponseBody {
                message_id: message.id,
                data,
              }),
            )?;
          }
          MessageBody::GetFileMatchingInfo(config_id) => {
            let Some(config) = configs.get(&config_id.as_raw()) else {
              send_error_response(
                &mut stdout_writer,
                &mut next_message_id,
                message.id,
                anyhow!("Did not find configuration for id: {}", config_id),
              )?;
              continue;
            };
            let data = serde_json::to_vec(&config.file_matching)?;
            send_response_body(
              &mut stdout_writer,
              &mut next_message_id,
              MessageBody::DataResponse(ResponseBody {
                message_id: message.id,
                data,
              }),
            )?;
          }
          MessageBody::GetResolvedConfig(config_id) => {
            let Some(config) = configs.get(&config_id.as_raw()) else {
              send_error_response(
                &mut stdout_writer,
                &mut next_message_id,
                message.id,
                anyhow!("Did not find configuration for id: {}", config_id),
              )?;
              continue;
            };
            let data = serde_json::to_vec(&*config.config)?;
            send_response_body(
              &mut stdout_writer,
              &mut next_message_id,
              MessageBody::DataResponse(ResponseBody {
                message_id: message.id,
                data,
              }),
            )?;
          }
          MessageBody::CheckConfigUpdates(body_bytes) => {
            let message_body = serde_json::from_slice::<CheckConfigUpdatesMessageBody>(&body_bytes)
              .context("Could not deserialize the check config updates message body.")?;
            let changes = handler.check_config_updates(message_body).await?;
            let data = serde_json::to_vec(&CheckConfigUpdatesResponseBody { changes })?;
            send_response_body(
              &mut stdout_writer,
              &mut next_message_id,
              MessageBody::DataResponse(ResponseBody {
                message_id: message.id,
                data,
              }),
            )?;
          }
          MessageBody::Format(body) => {
            let Some(stored_config) = configs.get(&body.config_id.as_raw()).cloned() else {
              send_error_response(
                &mut stdout_writer,
                &mut next_message_id,
                message.id,
                anyhow!("Did not find configuration for id: {}", body.config_id),
              )?;
              continue;
            };

            let config = if body.override_config.is_empty() {
              stored_config.config.clone()
            } else {
              let mut config_map = stored_config.config_map.clone();
              let override_config_map: ConfigKeyMap = serde_json::from_slice(&body.override_config)?;
              for (key, value) in override_config_map {
                config_map.insert(key, value);
              }
              Arc::new(
                handler
                  .resolve_config(config_map, stored_config.global_config.clone())
                  .await
                  .config,
              )
            };

            let message_id = message.id;
            let token = CancellationToken::new();
            format_tokens.insert(message_id, token.clone());
            let request = FormatRequest {
              file_path: body.file_path,
              range: body.range,
              config_id: body.config_id,
              config,
              file_bytes: body.file_bytes,
              token: Arc::new(token),
            };

            let handler_ref = &handler;
            in_flight.push(async move {
              let result = handler_ref
                .format(request, |_request: HostFormatRequest| {
                  async { Err(anyhow!("Host formatting is not supported by this plugin.")) }
                    .boxed_local()
                })
                .await;
              (message_id, result)
            });
          }
          MessageBody::CancelFormat(message_id) => {
            // Cooperative cancellation: signal the in-flight token. Removing it
            // here makes the completion arm suppress the response, matching the
            // protocol (a cancelled format gets no reply).
            if let Some(token) = format_tokens.remove(&message_id) {
              token.cancel();
            }
          }
          MessageBody::Success(_)
          | MessageBody::DataResponse(_)
          | MessageBody::Error(_)
          | MessageBody::FormatResponse(_)
          | MessageBody::HostFormat
          | MessageBody::Unknown(_) => {
            let error_text = match message.body {
              MessageBody::Unknown(message_kind) => {
                format!("Unknown CLI to plugin message kind: {message_kind}.")
              }
              _ => "Unsupported CLI to plugin message.".to_string(),
            };
            send_error_response(
              &mut stdout_writer,
              &mut next_message_id,
              message.id,
              anyhow!(error_text),
            )?;
          }
        }
      }

      Some((message_id, result)) = in_flight.next(), if !in_flight.is_empty() => {
        // Only respond if the host did not cancel: `CancelFormat` removes the
        // token, so a missing entry means the format was cancelled and the
        // protocol expects no reply.
        if format_tokens.remove(&message_id).is_some() {
          let body = match result {
            Ok(text) => MessageBody::FormatResponse(ResponseBody {
              message_id,
              data: text,
            }),
            Err(err) => MessageBody::Error(ResponseBody {
              message_id,
              data: format!("{:#}", err).into_bytes(),
            }),
          };
          send_response_body(&mut stdout_writer, &mut next_message_id, body)?;
        }
      }
    }
  }
}

fn send_response_body<TWrite: std::io::Write + Unpin>(
  stdout_writer: &mut MessageWriter<TWrite>,
  next_message_id: &mut u32,
  body: MessageBody,
) -> Result<()> {
  let message = ProcessPluginMessage {
    id: *next_message_id,
    body,
  };
  *next_message_id = next_message_id.saturating_add(1);
  message.write(stdout_writer)?;
  Ok(())
}

fn send_error_response<TWrite: std::io::Write + Unpin>(
  stdout_writer: &mut MessageWriter<TWrite>,
  next_message_id: &mut u32,
  original_message_id: u32,
  err: deno_core::anyhow::Error,
) -> Result<()> {
  send_response_body(
    stdout_writer,
    next_message_id,
    MessageBody::Error(ResponseBody {
      message_id: original_message_id,
      data: format!("{:#}", err).into_bytes(),
    }),
  )
}

fn schema_establishment_phase<TRead: std::io::Read + Unpin, TWrite: std::io::Write + Unpin>(
  stdin: &mut MessageReader<TRead>,
  stdout: &mut MessageWriter<TWrite>,
) -> Result<()> {
  if stdin.read_u32()? != 0 {
    bail!("Expected a schema version request of `0`.");
  }

  stdout.send_u32(0)?;
  stdout.send_u32(PLUGIN_SCHEMA_VERSION)?;
  stdout.flush()?;

  Ok(())
}

#[cfg(test)]
mod tests {
  use std::io::Cursor;
  use std::sync::Arc;
  use std::sync::Mutex;
  use std::time::Duration;

  use dprint_core::async_runtime::LocalBoxFuture;
  use dprint_core::async_runtime::async_trait;
  use dprint_core::configuration::ConfigKeyMap;
  use dprint_core::configuration::GlobalConfiguration;
  use dprint_core::plugins::FileMatchingInfo;
  use dprint_core::plugins::FormatRequest;
  use dprint_core::plugins::FormatResult;
  use dprint_core::plugins::HostFormatRequest;
  use dprint_core::plugins::PluginInfo;
  use dprint_core::plugins::PluginResolveConfigurationResult;
  use dprint_plugin_deno_base::util::create_tokio_runtime;

  use super::*;
  use crate::process_messages::FormatMessageBody;
  use crate::process_messages::RegisterConfigMessageBody;
  use dprint_core::plugins::FormatConfigId;

  /// Sentinel input that makes [`MockHandler::format`] block until cancelled.
  const WAIT_SENTINEL: &[u8] = b"WAIT";
  /// Sentinel input that makes [`MockHandler::format`] yield once before finishing.
  const YIELD_SENTINEL: &[u8] = b"YIELD";
  const FORMATTED_OUTPUT: &[u8] = b"FORMATTED";
  const TEST_TIMEOUT: Duration = Duration::from_secs(5);

  #[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
  struct MockConfig;

  /// A handler with no V8: a format whose input is [`WAIT_SENTINEL`] blocks on
  /// the cancellation token; any other input formats immediately. This lets the
  /// loop tests be deterministic — the "waiting" format only finishes once the
  /// token is cancelled.
  #[derive(Default)]
  struct MockHandler {
    start_cancelled_values: Arc<Mutex<Vec<bool>>>,
  }

  #[async_trait(?Send)]
  impl AsyncPluginHandler for MockHandler {
    type Configuration = MockConfig;

    fn plugin_info(&self) -> PluginInfo {
      PluginInfo {
        name: "mock".to_string(),
        version: "0.0.0".to_string(),
        config_key: "mock".to_string(),
        help_url: String::new(),
        config_schema_url: String::new(),
        update_url: None,
      }
    }

    fn license_text(&self) -> String {
      String::new()
    }

    async fn resolve_config(
      &self,
      _config: ConfigKeyMap,
      _global_config: GlobalConfiguration,
    ) -> PluginResolveConfigurationResult<Self::Configuration> {
      PluginResolveConfigurationResult {
        config: MockConfig,
        diagnostics: Vec::new(),
        file_matching: FileMatchingInfo {
          file_extensions: vec!["svg".to_string()],
          file_names: vec![],
        },
      }
    }

    async fn format(
      &self,
      request: FormatRequest<Self::Configuration>,
      _format_with_host: impl FnMut(HostFormatRequest) -> LocalBoxFuture<'static, FormatResult>
      + 'static,
    ) -> FormatResult {
      self
        .start_cancelled_values
        .lock()
        .unwrap()
        .push(request.token.is_cancelled());

      if request.file_bytes == WAIT_SENTINEL {
        request.token.wait_cancellation().await;
        Ok(None)
      } else if request.file_bytes == YIELD_SENTINEL {
        tokio::task::yield_now().await;
        Ok(Some(FORMATTED_OUTPUT.to_vec()))
      } else {
        Ok(Some(FORMATTED_OUTPUT.to_vec()))
      }
    }
  }

  /// A writer that captures everything into a shared buffer for inspection.
  #[derive(Clone)]
  struct SharedWriter(Arc<Mutex<Vec<u8>>>);

  impl std::io::Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
      self.0.lock().unwrap().extend_from_slice(buf);
      Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
      Ok(())
    }
  }

  fn register_config_msg(id: u32, config_id: u32) -> ProcessPluginMessage {
    ProcessPluginMessage {
      id,
      body: MessageBody::RegisterConfig(RegisterConfigMessageBody {
        config_id: FormatConfigId::from_raw(config_id),
        global_config: b"{}".to_vec(),
        plugin_config: b"{}".to_vec(),
      }),
    }
  }

  fn format_msg(id: u32, config_id: u32, file_bytes: &[u8]) -> ProcessPluginMessage {
    ProcessPluginMessage {
      id,
      body: MessageBody::Format(FormatMessageBody {
        file_path: "a.svg".into(),
        range: None,
        config_id: FormatConfigId::from_raw(config_id),
        override_config: Vec::new(),
        file_bytes: file_bytes.to_vec(),
      }),
    }
  }

  /// Parses the loop's output into message bodies.
  fn decode_output(output: Vec<u8>) -> Vec<MessageBody> {
    let mut reader = MessageReader::new(Cursor::new(output));
    let mut bodies = Vec::new();
    while let Ok(message) = ProcessPluginMessage::read(&mut reader) {
      bodies.push(message.body);
    }
    bodies
  }

  fn run_with_handler(
    handler: MockHandler,
    messages: Vec<ProcessPluginMessage>,
  ) -> Vec<MessageBody> {
    let output = Arc::new(Mutex::new(Vec::new()));
    let writer = SharedWriter(output.clone());
    let (message_tx, message_rx) =
      tokio::sync::mpsc::unbounded_channel::<std::io::Result<ProcessPluginMessage>>();
    for message in messages {
      message_tx.send(Ok(message)).unwrap();
    }
    drop(message_tx);

    let runtime = create_tokio_runtime();
    runtime.block_on(async {
      tokio::time::timeout(
        TEST_TIMEOUT,
        drive_message_loop(handler, MessageWriter::new(writer), message_rx),
      )
      .await
      .expect("process loop test timed out")
      .unwrap();
    });
    let bytes = output.lock().unwrap().clone();
    decode_output(bytes)
  }

  fn run(messages: Vec<ProcessPluginMessage>) -> Vec<MessageBody> {
    run_with_handler(MockHandler::default(), messages)
  }

  /// Encodes the schema handshake byte (`0`) followed by the given messages,
  /// exactly as the dprint CLI would send them.
  fn encode_input(messages: Vec<ProcessPluginMessage>) -> Vec<u8> {
    let mut buf = Vec::new();
    {
      let mut writer = MessageWriter::new(&mut buf);
      writer.send_u32(0).unwrap();
      for message in &messages {
        message.write(&mut writer).unwrap();
      }
    }
    buf
  }

  fn run_encoded_input(input: Vec<u8>) -> Vec<MessageBody> {
    let output = Arc::new(Mutex::new(Vec::new()));
    let writer = SharedWriter(output.clone());
    let runtime = create_tokio_runtime();
    runtime.block_on(async {
      tokio::time::timeout(
        TEST_TIMEOUT,
        run_message_loop(
          MockHandler::default(),
          MessageReader::new(Cursor::new(input)),
          MessageWriter::new(writer),
        ),
      )
      .await
      .expect("process loop test timed out")
      .unwrap();
    });

    let bytes = output.lock().unwrap().clone();
    let mut reader = MessageReader::new(Cursor::new(bytes));
    reader.read_u32().unwrap();
    reader.read_u32().unwrap();
    let mut bodies = Vec::new();
    while let Ok(message) = ProcessPluginMessage::read(&mut reader) {
      bodies.push(message.body);
    }
    bodies
  }

  #[test]
  fn encoded_loop_performs_schema_handshake() {
    let bodies = run_encoded_input(encode_input(vec![register_config_msg(1, 0)]));

    assert!(
      bodies
        .iter()
        .any(|body| matches!(body, MessageBody::Success(1))),
      "expected a Success for the RegisterConfig, got: {bodies:?}"
    );
  }

  #[test]
  fn cancel_format_suppresses_the_response() {
    let start_cancelled_values = Arc::new(Mutex::new(Vec::new()));
    let handler = MockHandler {
      start_cancelled_values: start_cancelled_values.clone(),
    };
    // Enqueue a format, then enqueue its cancel before the format is first
    // polled. The blocking format only completes after the token is cancelled,
    // so this covers pre-start cancellation plus response suppression.
    let messages = vec![
      register_config_msg(1, 0),
      format_msg(100, 0, WAIT_SENTINEL),
      ProcessPluginMessage {
        id: 2,
        body: MessageBody::CancelFormat(100),
      },
    ];

    let bodies = run_with_handler(handler, messages);

    // The register succeeded, proving the loop ran...
    assert!(
      bodies
        .iter()
        .any(|body| matches!(body, MessageBody::Success(1))),
      "expected a Success for the RegisterConfig, got: {bodies:?}"
    );
    // ...but the cancelled format produced no FormatResponse and no Error.
    assert!(
      !bodies.iter().any(|body| match body {
        MessageBody::FormatResponse(r) => r.message_id == 100,
        MessageBody::Error(r) => r.message_id == 100,
        _ => false,
      }),
      "expected no response for the cancelled format, got: {bodies:?}"
    );
    assert_eq!(
      *start_cancelled_values.lock().unwrap(),
      vec![true],
      "expected queued cancellation before format start"
    );
  }

  #[test]
  fn eof_drains_accepted_format_before_returning() {
    let bodies = run(vec![
      register_config_msg(1, 0),
      format_msg(100, 0, YIELD_SENTINEL),
    ]);

    let response = bodies.iter().find_map(|body| match body {
      MessageBody::FormatResponse(r) if r.message_id == 100 => Some(r.data.clone()),
      _ => None,
    });
    assert_eq!(
      response,
      Some(Some(FORMATTED_OUTPUT.to_vec())),
      "expected EOF to drain accepted format, got: {bodies:?}"
    );
  }

  #[test]
  fn format_without_cancel_sends_a_response() {
    let bodies = run(vec![
      register_config_msg(1, 0),
      format_msg(100, 0, b"hello"),
    ]);

    let response = bodies.iter().find_map(|body| match body {
      MessageBody::FormatResponse(r) if r.message_id == 100 => Some(r.data.clone()),
      _ => None,
    });
    assert_eq!(
      response,
      Some(Some(FORMATTED_OUTPUT.to_vec())),
      "expected the formatted output, got: {bodies:?}"
    );
  }
}

use std::collections::HashMap;
use std::io::ErrorKind;
use std::rc::Rc;
use std::sync::Arc;

use deno_core::anyhow::Context;
use deno_core::anyhow::Result;
use deno_core::anyhow::anyhow;
use deno_core::anyhow::bail;
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
use dprint_core::plugins::NullCancellationToken;
use serde::Serialize;

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
pub async fn handle_process_stdio_messages_sync<THandler: AsyncPluginHandler>(
  handler: THandler,
) -> Result<()> {
  dprint_core::plugins::process::setup_exit_process_panic_hook();

  let mut stdin_reader = MessageReader::new(std::io::stdin());
  let mut stdout_writer = MessageWriter::new(std::io::stdout());
  schema_establishment_phase(&mut stdin_reader, &mut stdout_writer)
    .context("Failed estabilishing schema.")?;

  let mut next_message_id = 1_u32;
  let mut configs = HashMap::<u32, Rc<StoredConfig<THandler::Configuration>>>::new();

  loop {
    let message = match ProcessPluginMessage::read(&mut stdin_reader) {
      Ok(message) => message,
      Err(err) if matches!(err.kind(), ErrorKind::UnexpectedEof | ErrorKind::BrokenPipe) => {
        return Ok(());
      }
      Err(err) => return Err(err.into()),
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

        let result = handler
          .format(
            FormatRequest {
              file_path: body.file_path,
              range: body.range,
              config_id: body.config_id,
              config,
              file_bytes: body.file_bytes,
              token: Arc::new(NullCancellationToken),
            },
            |_request: HostFormatRequest| {
              async { Err(anyhow!("Host formatting is not supported by this plugin.")) }
                .boxed_local()
            },
          )
          .await;

        let body = match result {
          Ok(text) => MessageBody::FormatResponse(ResponseBody {
            message_id: message.id,
            data: text,
          }),
          Err(err) => MessageBody::Error(ResponseBody {
            message_id: message.id,
            data: format!("{:#}", err).into_bytes(),
          }),
        };
        send_response_body(&mut stdout_writer, &mut next_message_id, body)?;
      }
      MessageBody::CancelFormat(_) => {
        // Cancellation is best-effort only. The current formatter path does not
        // observe tokens yet, so acknowledge to keep the protocol moving.
        send_response_body(
          &mut stdout_writer,
          &mut next_message_id,
          MessageBody::Success(message.id),
        )?;
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

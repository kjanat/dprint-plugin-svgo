use deno_core::Extension;
use deno_core::PollEventLoopOptions;
use deno_core::RuntimeOptions;
use deno_core::anyhow::Error;
use deno_core::anyhow::anyhow;
use deno_core::serde_v8;
use deno_core::v8;
use deno_core::v8::Platform;
use deno_core::v8::SharedRef;
use serde::de::DeserializeOwned;

fn get_platform() -> SharedRef<Platform> {
  static PLATFORM: std::sync::OnceLock<SharedRef<Platform>> = std::sync::OnceLock::new();

  PLATFORM
    .get_or_init(|| v8::new_default_platform(1, false).make_shared())
    .clone()
}

pub struct CreateRuntimeOptions {
  pub extensions: Vec<Extension>,
  pub startup_snapshot: Option<&'static [u8]>,
}

pub struct JsRuntime {
  inner: deno_core::JsRuntime,
}

impl JsRuntime {
  #[must_use]
  pub fn new(options: CreateRuntimeOptions) -> Self {
    Self {
      inner: deno_core::JsRuntime::new(RuntimeOptions {
        startup_snapshot: options.startup_snapshot,
        v8_platform: Some(get_platform()),
        extensions: options.extensions,
        ..Default::default()
      }),
    }
  }

  /// Call this once on the main thread.
  pub fn initialize_main_thread() {
    deno_core::JsRuntime::init_platform(Some(get_platform()));
  }

  /// Executes a format script and returns the formatted output.
  ///
  /// # Errors
  ///
  /// Returns an error if script execution fails or the result cannot be deserialized.
  pub async fn execute_format_script(&mut self, code: String) -> Result<Option<String>, Error> {
    let global = self.inner.execute_script("format.js", code)?;
    let resolve = self.inner.resolve(global);
    let global = self
      .inner
      .with_event_loop_promise(resolve, PollEventLoopOptions::default())
      .await?;
    deno_core::scope!(scope, self.inner);
    let local = v8::Local::new(scope, global);
    if local.is_undefined() {
      Ok(None)
    } else {
      let deserialized_value = serde_v8::from_v8::<String>(scope, local);
      match deserialized_value {
        Ok(value) => Ok(Some(value)),
        Err(err) => Err(anyhow!("Cannot deserialize serde_v8 value: {:#}", err)),
      }
    }
  }

  /// Executes a script by name.
  ///
  /// # Errors
  ///
  /// Returns an error if script execution fails.
  pub fn execute_script(&mut self, script_name: &'static str, code: String) -> Result<(), Error> {
    self.inner.execute_script(script_name, code)?;
    Ok(())
  }

  /// Executes an async function and returns the deserialized result.
  ///
  /// # Errors
  ///
  /// Returns an error if script execution fails, the function call fails,
  /// or the result cannot be deserialized.
  pub async fn execute_async_fn<T>(
    &mut self,
    script_name: &'static str,
    fn_name: String,
  ) -> Result<T, Error>
  where
    T: DeserializeOwned,
  {
    let inner = &mut self.inner;
    let fn_value = inner.execute_script(script_name, fn_name)?;
    let resolve = inner.resolve(fn_value);
    let fn_value = inner
      .with_event_loop_promise(resolve, PollEventLoopOptions::default())
      .await?;
    let fn_func = {
      deno_core::scope!(scope, inner);
      let fn_func: v8::Local<v8::Function> = v8::Local::new(scope, fn_value).try_into()?;
      v8::Global::new(scope, fn_func)
    };
    let call = inner.call(&fn_func);
    let result = inner
      .with_event_loop_promise(call, PollEventLoopOptions::default())
      .await?;
    deno_core::scope!(scope, inner);
    let local = v8::Local::new(scope, result);
    Ok(serde_v8::from_v8::<T>(scope, local)?)
  }
}

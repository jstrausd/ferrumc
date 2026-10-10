//! Optional, process-isolated Java plugin transport. Rust retains authority over
//! identity, permissions, recipients, world state, and every network connection.

use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;
use tokio::time::{sleep_until, timeout_at, Instant};

/// Hard transport cap, checked before allocation or queue admission.
const MAX_FRAME: usize = 65_536;
/// At most 32 serialized requests (~2 MiB) wait behind the single Java callback.
const QUEUE_CAPACITY: usize = 32;
const MAX_PLUGINS: usize = 64;
const MAX_COMMANDS: usize = 256;
const MAX_ALIASES: usize = 16;
const MAX_MESSAGES: usize = 128;

/// Classified bridge failures; transport corruption terminates the child actor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BridgeError {
    Io(String),
    Protocol(String),
    Remote(String),
    Timeout,
    Cancelled,
    QueueFull,
    Stopped,
}

impl fmt::Display for BridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "Java bridge I/O failure: {error}"),
            Self::Protocol(error) => write!(f, "invalid Java bridge response: {error}"),
            Self::Remote(error) => write!(f, "Java plugin failed: {error}"),
            Self::Timeout => f.write_str("Java bridge RPC deadline expired"),
            Self::Cancelled => f.write_str("Java bridge RPC was cancelled"),
            Self::QueueFull => f.write_str("Java bridge request queue is full"),
            Self::Stopped => f.write_str("Java bridge process is unavailable"),
        }
    }
}

impl std::error::Error for BridgeError {}

impl From<std::io::Error> for BridgeError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

/// Rust-derived sender identity; clients and Java never supply these privileges.
#[derive(Debug, Serialize)]
pub(crate) struct JavaPlayer {
    pub(crate) name: String,
    pub(crate) uuid: String,
    pub(crate) op: bool,
    pub(crate) permissions: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "op", rename_all = "lowercase")]
enum Operation {
    Load {
        plugins_dir: String,
    },
    Command {
        command: String,
        args: Vec<String>,
        player: JavaPlayer,
    },
    Join {
        player: JavaPlayer,
    },
    Quit {
        player: JavaPlayer,
    },
    Shutdown,
}

impl Operation {
    fn recipient(&self) -> Option<&str> {
        match self {
            Self::Command { player, .. } | Self::Join { player } | Self::Quit { player } => {
                Some(&player.uuid)
            }
            Self::Load { .. } | Self::Shutdown => None,
        }
    }
}

#[derive(Serialize)]
struct WireRequest<'a> {
    id: u64,
    #[serde(flatten)]
    operation: &'a Operation,
}

/// Validated, bounded command metadata advertised by a loaded Java plugin.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct JavaCommand {
    pub(crate) name: String,
    description: String,
    pub(crate) permission: Option<String>,
    pub(crate) aliases: Vec<String>,
    #[serde(skip)]
    routes: Vec<String>,
}

impl JavaCommand {
    /// Plain and plugin-namespaced routes published to the Rust command graph.
    pub(crate) fn routes(&self) -> &[String] {
        &self.routes
    }

    /// Derives the standard lowercase plugin namespace without wire metadata.
    pub(crate) fn bind_namespace(&mut self, plugin: &str) {
        let namespace = plugin.to_ascii_lowercase();
        self.routes = std::iter::once(&self.name)
            .chain(&self.aliases)
            .flat_map(|label| [label.clone(), format!("{namespace}:{label}")])
            .collect();
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct JavaPlugin {
    name: String,
    version: String,
    commands: Vec<JavaCommand>,
}

/// A validated message directed only to the requesting player or the console.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct JavaMessage {
    pub(crate) player: Option<String>,
    pub(crate) text: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Response {
    id: u64,
    ok: bool,
    #[serde(default)]
    plugins: Vec<JavaPlugin>,
    messages: Vec<JavaMessage>,
    error: Option<String>,
}

impl Response {
    fn validate(&self, id: u64, recipient: Option<&str>, load: bool) -> Result<(), BridgeError> {
        ensure(
            self.id == id,
            "response ID does not match the active request",
        )?;
        ensure(self.messages.len() <= MAX_MESSAGES, "too many messages")?;
        ensure(self.plugins.len() <= MAX_PLUGINS, "too many plugins")?;
        ensure(
            load || self.plugins.is_empty(),
            "metadata outside load response",
        )?;
        ensure(
            !self.ok || self.error.is_none(),
            "successful response contains an error",
        )?;
        if let Some(error) = &self.error {
            bounded(error, 2_048, "error")?;
        }
        for message in &self.messages {
            bounded(&message.text, 4_096, "message text")?;
            if let Some(player) = &message.player {
                ensure(
                    Some(player.as_str()) == recipient,
                    "message targets a player outside this request",
                )?;
            }
        }
        let mut names = BTreeSet::new();
        let mut labels = BTreeSet::new();
        let mut commands = 0usize;
        let mut aliases = 0usize;
        for plugin in &self.plugins {
            bounded(&plugin.name, 128, "plugin name")?;
            bounded(&plugin.version, 128, "plugin version")?;
            ensure(
                !plugin.name.is_empty() && names.insert(plugin.name.to_ascii_lowercase()),
                "empty or duplicate plugin name",
            )?;
            ensure(
                plugin
                    .name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.')),
                "plugin name is not an ASCII namespace",
            )?;
            commands += plugin.commands.len();
            ensure(commands <= MAX_COMMANDS, "too many commands")?;
            for command in &plugin.commands {
                command_label(&command.name)?;
                ensure(
                    labels.insert(command.name.to_ascii_lowercase()),
                    "duplicate command or alias",
                )?;
                bounded(&command.description, 512, "command description")?;
                if let Some(permission) = &command.permission {
                    bounded(permission, 128, "command permission")?;
                    ensure(
                        ferrumc_permission::PermissionNode::parse(permission).is_ok(),
                        "invalid permission node",
                    )?;
                }
                ensure(
                    command.aliases.len() <= MAX_ALIASES,
                    "too many aliases on a command",
                )?;
                aliases += command.aliases.len();
                ensure(aliases <= MAX_COMMANDS, "too many aliases")?;
                for alias in &command.aliases {
                    command_label(alias)?;
                    ensure(
                        labels.insert(alias.to_ascii_lowercase()),
                        "duplicate command or alias",
                    )?;
                }
            }
        }
        Ok(())
    }

    fn into_messages(self) -> Result<Vec<JavaMessage>, BridgeError> {
        if self.ok {
            Ok(self.messages)
        } else {
            Err(BridgeError::Remote(self.error.unwrap_or_else(|| {
                "plugin callback rejected the request".to_owned()
            })))
        }
    }
}

fn ensure(condition: bool, message: &str) -> Result<(), BridgeError> {
    if condition {
        Ok(())
    } else {
        Err(BridgeError::Protocol(message.to_owned()))
    }
}

fn bounded(text: &str, max: usize, field: &str) -> Result<(), BridgeError> {
    ensure(
        text.len() <= max && !text.contains('\0'),
        &format!("{field} exceeds its bound or contains NUL"),
    )
}

fn command_label(label: &str) -> Result<(), BridgeError> {
    bounded(label, 64, "command label")?;
    ensure(
        !label.is_empty()
            && label.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'_' | b'-' | b':' | b'.')
            }),
        "command labels must be lowercase ASCII literals",
    )
}

struct QueuedRequest {
    id: u64,
    bytes: Vec<u8>,
    deadline: Instant,
    recipient: Option<String>,
    load: bool,
    shutdown: bool,
    reply: oneshot::Sender<Result<Response, BridgeError>>,
}

/// Cheap connection-side handle. The actor owns the process and all pipe I/O.
#[derive(Debug, Clone)]
pub(crate) struct JavaBridge {
    sender: mpsc::Sender<QueuedRequest>,
    next_id: Arc<AtomicU64>,
    timeout: Duration,
    commands: Arc<Vec<JavaCommand>>,
}

impl JavaBridge {
    /// Validated command descriptors, including aliases, frozen after load.
    pub(crate) fn commands(&self) -> &[JavaCommand] {
        &self.commands
    }

    /// Resolves a literal or alias without sending work to Java.
    pub(crate) fn find_command(&self, label: &str) -> Option<&JavaCommand> {
        self.commands.iter().find(|command| {
            command
                .routes
                .iter()
                .any(|route| route.eq_ignore_ascii_case(label))
        })
    }

    /// Executes one command with Rust-derived sender privileges.
    pub(crate) async fn command(
        &self,
        label: &str,
        args: Vec<String>,
        player: JavaPlayer,
    ) -> Result<Vec<JavaMessage>, BridgeError> {
        ensure(args.len() <= 64, "too many command arguments")?;
        for argument in &args {
            bounded(argument, 512, "command argument")?;
        }
        self.rpc(Operation::Command {
            command: label.to_owned(),
            args,
            player,
        })
        .await?
        .into_messages()
    }

    /// Dispatches a real player lifecycle event after Play admission / on leave.
    pub(crate) async fn event(
        &self,
        joined: bool,
        player: JavaPlayer,
    ) -> Result<Vec<JavaMessage>, BridgeError> {
        self.rpc(if joined {
            Operation::Join { player }
        } else {
            Operation::Quit { player }
        })
        .await?
        .into_messages()
    }

    async fn rpc(&self, operation: Operation) -> Result<Response, BridgeError> {
        let deadline = Instant::now() + self.timeout;
        let id = self
            .next_id
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| BridgeError::Stopped)?;
        let bytes = serde_json::to_vec(&WireRequest {
            id,
            operation: &operation,
        })
        .map_err(|error| BridgeError::Protocol(error.to_string()))?;
        ensure(
            !bytes.is_empty() && bytes.len() <= MAX_FRAME,
            "outbound frame exceeds transport cap",
        )?;
        let (reply, response) = oneshot::channel();
        self.sender
            .try_send(QueuedRequest {
                id,
                bytes,
                deadline,
                recipient: operation.recipient().map(str::to_owned),
                load: matches!(operation, Operation::Load { .. }),
                shutdown: matches!(operation, Operation::Shutdown),
                reply,
            })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => BridgeError::QueueFull,
                mpsc::error::TrySendError::Closed(_) => BridgeError::Stopped,
            })?;
        timeout_at(deadline, response)
            .await
            .map_err(|_| BridgeError::Timeout)?
            .map_err(|_| BridgeError::Stopped)?
    }
}

/// Server-owned process lifecycle. Explicit shutdown awaits kill/reap; dropping
/// startup/admission futures signals the actor to kill/reap instead of leaving a
/// callback or child behind. Child `kill_on_drop` is a final runtime-abort fallback.
#[derive(Debug)]
pub(crate) struct JavaBridgeRuntime {
    bridge: JavaBridge,
    stop: watch::Sender<bool>,
    task: Option<JoinHandle<Result<(), BridgeError>>>,
}

impl JavaBridgeRuntime {
    /// Starts only the compatibility host, loads/validates all metadata before
    /// clients are accepted, and reaps the child if any startup RPC fails.
    pub(crate) async fn start(
        executable: &str,
        jar: &Path,
        plugins: &Path,
        timeout: Duration,
    ) -> Result<Self, BridgeError> {
        let mut process = Command::new(executable);
        process
            .arg("-jar")
            .arg(jar)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);
        let child = process.spawn()?;
        Self::start_child(child, plugins, timeout).await
    }

    async fn spawn_actor(mut child: Child, timeout: Duration) -> Result<Self, BridgeError> {
        let Some((input, output)) = child.stdin.take().zip(child.stdout.take()) else {
            let _ = child.start_kill();
            let _ = child.wait().await;
            return Err(BridgeError::Stopped);
        };
        let (sender, requests) = mpsc::channel(QUEUE_CAPACITY);
        let (stop, cancellation) = watch::channel(false);
        let task = tokio::spawn(run_actor(child, input, output, requests, cancellation));
        Ok(Self {
            bridge: JavaBridge {
                sender,
                next_id: Arc::new(AtomicU64::new(1)),
                timeout,
                commands: Arc::new(Vec::new()),
            },
            stop,
            task: Some(task),
        })
    }

    async fn start_child(
        mut child: Child,
        plugins: &Path,
        timeout: Duration,
    ) -> Result<Self, BridgeError> {
        let Some(plugins_dir) = plugins.to_str().map(str::to_owned) else {
            let _ = child.start_kill();
            let _ = child.wait().await;
            return Err(BridgeError::Protocol("plugin path is not UTF-8".to_owned()));
        };
        let mut runtime = Self::spawn_actor(child, timeout).await?;
        let loaded = runtime.bridge.rpc(Operation::Load { plugins_dir }).await;
        match loaded {
            Ok(response) if response.ok => {
                for message in &response.messages {
                    tracing::info!(message = %message.text, "Java plugin console");
                }
                runtime.bridge.commands = Arc::new(response.plugins.into_iter().flat_map(|plugin| {
                    tracing::info!(name = %plugin.name, version = %plugin.version, "loaded Java plugin");
                    plugin.commands.into_iter().map(move |mut command| {
                        command.bind_namespace(&plugin.name);
                        command
                    })
                }).collect());
                Ok(runtime)
            }
            outcome => {
                let error = match outcome {
                    Ok(response) => BridgeError::Remote(
                        response
                            .error
                            .unwrap_or_else(|| "plugin load failed".to_owned()),
                    ),
                    Err(error) => error,
                };
                let _ = runtime.stop.send(true);
                if let Some(task) = runtime.task.take() {
                    let _ = task.await;
                }
                Err(error)
            }
        }
    }

    /// Connection-side handle; metadata and process queues remain bounded.
    pub(crate) fn bridge(&self) -> JavaBridge {
        self.bridge.clone()
    }

    /// Sends exactly one shutdown request, then observes process completion.
    pub(crate) async fn shutdown(mut self) -> Result<(), BridgeError> {
        let rpc = self
            .bridge
            .rpc(Operation::Shutdown)
            .await
            .and_then(Response::into_messages);
        if rpc.is_err() {
            let _ = self.stop.send(true);
        }
        let result = if let Some(task) = self.task.take() {
            task.await
                .map_err(|error| BridgeError::Io(error.to_string()))?
        } else {
            Ok(())
        };
        result?;
        rpc.map(|messages| {
            for message in messages {
                tracing::info!(message = %message.text, "Java plugin console");
            }
        })
    }
}

impl Drop for JavaBridgeRuntime {
    fn drop(&mut self) {
        let _ = self.stop.send(true);
    }
}

async fn exchange<W: AsyncWrite + Unpin, R: AsyncRead + Unpin>(
    input: &mut W,
    output: &mut R,
    bytes: &[u8],
    id: u64,
    recipient: Option<&str>,
    load: bool,
) -> Result<Response, BridgeError> {
    let length = u32::try_from(bytes.len())
        .map_err(|_| BridgeError::Protocol("outbound frame length overflow".to_owned()))?;
    input.write_all(&length.to_be_bytes()).await?;
    input.write_all(bytes).await?;
    input.flush().await?;
    let length = output.read_u32().await? as usize;
    ensure(
        (1..=MAX_FRAME).contains(&length),
        "inbound frame length exceeds transport cap",
    )?;
    let mut bytes = vec![0u8; length];
    output.read_exact(&mut bytes).await?;
    let response: Response =
        serde_json::from_slice(&bytes).map_err(|error| BridgeError::Protocol(error.to_string()))?;
    response.validate(id, recipient, load)?;
    Ok(response)
}

async fn run_actor(
    mut child: Child,
    mut input: tokio::process::ChildStdin,
    mut output: tokio::process::ChildStdout,
    mut requests: mpsc::Receiver<QueuedRequest>,
    mut stop: watch::Receiver<bool>,
) -> Result<(), BridgeError> {
    let result = async { loop {
        let request = tokio::select! {
            biased;
            _ = stop.changed() => break Ok(()),
            status = child.wait() => break Err(BridgeError::Io(format!("bridge exited: {}", status?))),
            request = requests.recv() => match request { Some(request) => request, None => break Ok(()) },
        };
        let mut request = request;
        if request.reply.is_closed() || Instant::now() >= request.deadline {
            let _ = request.reply.send(Err(BridgeError::Timeout));
            continue;
        }
        // Pipe transactions cannot be partially abandoned: a timeout or dropped
        // caller terminates the process so delayed output cannot satisfy a later ID.
        let outcome = tokio::select! {
            biased;
            _ = stop.changed() => Err(BridgeError::Cancelled),
            () = sleep_until(request.deadline) => Err(BridgeError::Timeout),
            () = request.reply.closed() => Err(BridgeError::Cancelled),
            response = exchange(&mut input, &mut output, &request.bytes, request.id, request.recipient.as_deref(), request.load) => response,
        };
        let failed = outcome.as_ref().err().cloned();
        let shutdown = request.shutdown;
        let deadline = request.deadline;
        let _ = request.reply.send(outcome);
        if let Some(error) = failed { break Err(error); }
        if shutdown {
            break tokio::select! {
                biased;
                _ = stop.changed() => Err(BridgeError::Cancelled),
                () = sleep_until(deadline) => Err(BridgeError::Timeout),
                status = child.wait() => status.map_err(BridgeError::from).and_then(|status| {
                    if status.success() { Ok(()) } else { Err(BridgeError::Io(format!("bridge shutdown exited: {status}"))) }
                }),
            };
        }
    } }.await;
    // Always reap, including failure, cancellation, load rejection and explicit
    // stop. There is no extra detached cleanup task and no mutex across awaits.
    let _ = child.start_kill();
    child.wait().await?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const PLAYER: &str = "01234567-89ab-cdef-0123-456789abcdef";

    fn sender() -> JavaPlayer {
        JavaPlayer {
            name: "BridgeProbe".to_owned(),
            uuid: PLAYER.to_owned(),
            op: false,
            permissions: Vec::new(),
        }
    }

    /// Socket-free pipe fixture: consume the exact emitted request, then supply
    /// one framed response. This uses the real transport parser and validation.
    async fn wire_fixture(payload: Vec<u8>, length: u32) -> Result<Response, BridgeError> {
        let (local, mut peer) = tokio::io::duplex(MAX_FRAME * 2);
        let (mut output, mut input) = tokio::io::split(local);
        let writer = tokio::spawn(async move {
            let request_len = peer.read_u32().await.expect("request length");
            let mut request = vec![0u8; request_len as usize];
            peer.read_exact(&mut request).await.expect("request bytes");
            assert_eq!(request, b"{}");
            peer.write_all(&length.to_be_bytes())
                .await
                .expect("response length");
            peer.write_all(&payload).await.expect("response bytes");
        });
        let response = exchange(&mut input, &mut output, b"{}", 7, Some(PLAYER), false).await;
        writer.await.expect("fixture writer finished");
        response
    }

    #[tokio::test]
    async fn framing_rejects_wrong_ids_invalid_utf8_truncation_and_oversize_before_body_read() {
        let valid = serde_json::to_vec(
            &json!({"id":7,"ok":true,"messages":[{"player":PLAYER,"text":"hello"}],"error":null}),
        )
        .expect("JSON");
        assert_eq!(
            wire_fixture(valid.clone(), valid.len() as u32)
                .await
                .expect("valid response")
                .messages[0]
                .text,
            "hello"
        );
        let wrong_id = serde_json::to_vec(&json!({"id":8,"ok":true,"messages":[],"error":null}))
            .expect("JSON");
        assert!(matches!(
            wire_fixture(wrong_id.clone(), wrong_id.len() as u32).await,
            Err(BridgeError::Protocol(_))
        ));
        assert!(matches!(
            wire_fixture(vec![0xff], 1).await,
            Err(BridgeError::Protocol(_))
        ));
        assert!(matches!(
            wire_fixture(vec![b'{'], 2).await,
            Err(BridgeError::Io(_))
        ));
        for length in [0, MAX_FRAME as u32 + 1, u32::MAX] {
            assert!(matches!(
                wire_fixture(Vec::new(), length).await,
                Err(BridgeError::Protocol(_))
            ));
        }
    }

    #[test]
    fn response_validation_rejects_spoofed_recipients_unknown_fields_and_bounded_metadata() {
        let parse = |value: serde_json::Value| {
            serde_json::from_value::<Response>(value).expect("response JSON")
        };
        let targeted = parse(
            json!({"id":7,"ok":true,"messages":[{"player":"other-player","text":"spoof"}],"error":null}),
        );
        assert!(matches!(
            targeted.validate(7, Some(PLAYER), false),
            Err(BridgeError::Protocol(_))
        ));
        let excessive = parse(
            json!({"id":7,"ok":true,"messages":[{"player":null,"text":"x".repeat(4097)}],"error":null}),
        );
        assert!(excessive.validate(7, Some(PLAYER), false).is_err());
        let messages = parse(
            json!({"id":7,"ok":true,"messages":vec![json!({"player":null,"text":"x"});MAX_MESSAGES+1],"error":null}),
        );
        assert!(messages.validate(7, Some(PLAYER), false).is_err());
        assert!(serde_json::from_value::<Response>(
            json!({"id":7,"ok":true,"messages":[],"error":null,"unknown":true})
        )
        .is_err());
        for commands in [
            json!([{"name":"hello","description":"greeting","permission":null,"aliases":["hello"]}]),
            json!([{"name":"bad label","description":"greeting","permission":null,"aliases":[]}]),
            json!([{"name":"hello","description":"greeting","permission":null,"aliases":vec!["alias";MAX_ALIASES+1]}]),
        ] {
            let load = parse(
                json!({"id":7,"ok":true,"plugins":[{"name":"Fixture","version":"1","commands":commands}],"messages":[],"error":null}),
            );
            assert!(load.validate(7, None, true).is_err());
        }
        let failed =
            parse(json!({"id":7,"ok":false,"messages":[],"error":"unsupported world API"}));
        assert_eq!(
            failed.into_messages().err(),
            Some(BridgeError::Remote("unsupported world API".to_owned()))
        );
    }

    #[test]
    fn plugin_versions_use_utf8_byte_bounds_and_command_routes_keep_standard_aliases() {
        let mut response: Response = serde_json::from_value(json!({
            "id":7, "ok":true,
            "plugins":[{"name":"Fixture", "version":"é".repeat(64),
                "commands":[{"name":"fixture.cmd", "description":"", "permission":null,
                    "aliases":["falias"]}]}],
            "messages":[], "error":null
        }))
        .expect("metadata JSON");
        response
            .validate(7, None, true)
            .expect("exact UTF-8 bound and dotted label");
        let mut command = response.plugins[0].commands[0].clone();
        command.bind_namespace("Fixture");
        let (queue, _) = mpsc::channel(1);
        let bridge = JavaBridge {
            sender: queue,
            next_id: Arc::new(AtomicU64::new(1)),
            timeout: Duration::from_secs(1),
            commands: Arc::new(vec![command]),
        };
        for route in [
            "fixture.cmd",
            "FALIAS",
            "fixture:fixture.cmd",
            "FIXTURE:FALIAS",
        ] {
            assert!(bridge.find_command(route).is_some(), "{route}");
        }
        assert!(bridge.find_command("other:falias").is_none());
        response.plugins[0].version.push('x');
        assert!(response.validate(7, None, true).is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn rpc_budget_includes_queue_time_and_full_queue_refuses_admission() {
        let (queue, mut inbox) = mpsc::channel(1);
        let bridge = JavaBridge {
            sender: queue,
            next_id: Arc::new(AtomicU64::new(1)),
            timeout: Duration::from_secs(1),
            commands: Arc::new(Vec::new()),
        };
        let first_bridge = bridge.clone();
        let first =
            tokio::spawn(
                async move { first_bridge.rpc(Operation::Join { player: sender() }).await },
            );
        // Inspect the request held out of execution, so this cannot accidentally
        // test a child I/O timeout instead of the absolute queued-request budget.
        let held = inbox.recv().await.expect("queued request");
        bridge
            .sender
            .try_send(held)
            .expect("put it back into the full bounded queue");
        assert_eq!(
            bridge.rpc(Operation::Join { player: sender() }).await.err(),
            Some(BridgeError::QueueFull)
        );
        tokio::time::advance(Duration::from_secs(1)).await;
        assert_eq!(
            first.await.expect("request completed").err(),
            Some(BridgeError::Timeout)
        );
        assert!(inbox
            .recv()
            .await
            .expect("original request")
            .reply
            .is_closed());
    }

    #[cfg(unix)]
    fn test_child(script: &str) -> Child {
        Command::new("sh")
            .arg("-c")
            .arg(script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .expect("POSIX process fixture")
    }

    #[cfg(target_os = "linux")]
    fn assert_reaped(pid: u32) {
        assert!(
            !Path::new(&format!("/proc/{pid}")).exists(),
            "bridge child is killed and reaped"
        );
    }

    #[cfg(unix)]
    #[tokio::test(start_paused = true)]
    async fn stalled_startup_deadline_kills_and_reaps_the_child() {
        let child = test_child("while IFS= read -r line; do :; done");
        #[cfg(target_os = "linux")]
        let pid = child.id().expect("child ID");
        let error =
            JavaBridgeRuntime::start_child(child, Path::new("plugins"), Duration::from_secs(1))
                .await
                .expect_err("silent process must time out");
        assert_eq!(error, BridgeError::Timeout);
        #[cfg(target_os = "linux")]
        assert_reaped(pid);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn crashing_startup_is_reported_and_reaped() {
        let child = test_child("exit 7");
        #[cfg(target_os = "linux")]
        let pid = child.id().expect("child ID");
        let error =
            JavaBridgeRuntime::start_child(child, Path::new("plugins"), Duration::from_secs(5))
                .await
                .expect_err("crashed process must fail startup");
        assert!(matches!(error, BridgeError::Io(_)));
        #[cfg(target_os = "linux")]
        assert_reaped(pid);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn cancellation_during_an_active_pipe_transaction_kills_and_reaps() {
        // The one-byte stderr marker acknowledges actual request consumption.
        // Cancelling before that marker would test queued work, not active I/O.
        let mut child =
            test_child("dd bs=1 count=1 of=/dev/null 2>/dev/null; printf x >&2; read line");
        #[cfg(target_os = "linux")]
        let pid = child.id().expect("child ID");
        let mut marker = child.stderr.take().expect("stderr marker pipe");
        let mut runtime = JavaBridgeRuntime::spawn_actor(child, Duration::from_secs(5))
            .await
            .expect("actor started");
        let bridge = runtime.bridge();
        let caller =
            tokio::spawn(async move { bridge.rpc(Operation::Join { player: sender() }).await });
        assert_eq!(
            marker.read_u8().await.expect("child received request"),
            b'x'
        );
        caller.abort();
        assert!(caller.await.expect_err("caller cancelled").is_cancelled());
        assert_eq!(
            runtime
                .task
                .take()
                .expect("owned actor")
                .await
                .expect("actor joined")
                .err(),
            Some(BridgeError::Cancelled)
        );
        #[cfg(target_os = "linux")]
        assert_reaped(pid);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn dropping_runtime_cancels_the_post_shutdown_process_wait() {
        let response = r#"{"id":1,"ok":true,"messages":[],"error":null}"#;
        let request = serde_json::to_vec(&WireRequest {
            id: 1,
            operation: &Operation::Shutdown,
        })
        .expect("request JSON");
        let length = (response.len() as u32).to_be_bytes();
        let prefix = length.iter().fold(String::new(), |mut prefix, byte| {
            use std::fmt::Write as _;
            write!(prefix, "\\{byte:03o}").expect("write length octet");
            prefix
        });
        let script = format!(
            "dd bs=1 count={} of=/dev/null 2>/dev/null; printf '{prefix}'; printf '%s' '{response}'; read line",
            request.len() + 4
        );
        let child = test_child(&script);
        #[cfg(target_os = "linux")]
        let pid = child.id().expect("child ID");
        let mut runtime = JavaBridgeRuntime::spawn_actor(child, Duration::from_secs(30))
            .await
            .expect("actor started");
        runtime
            .bridge
            .rpc(Operation::Shutdown)
            .await
            .expect("shutdown acknowledged");
        let actor = runtime.task.take().expect("owned actor");
        drop(runtime);
        assert_eq!(
            actor.await.expect("actor joined").err(),
            Some(BridgeError::Cancelled)
        );
        #[cfg(target_os = "linux")]
        assert_reaped(pid);
    }
}

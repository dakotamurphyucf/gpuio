use crate::response;
use async_channel::Receiver;
use futures_lite::{StreamExt, future};
use gpuio_protocol::file_dialog::*;
use std::{collections::HashMap, time::Duration};
use zbus::{
    Connection, MatchRule, MessageStream,
    message::Type,
    zvariant::{OwnedObjectPath, Value},
};

const DESTINATION: &str = "org.freedesktop.portal.Desktop";
const DESKTOP: &str = "/org/freedesktop/portal/desktop";
const CHOOSER: &str = "org.freedesktop.portal.FileChooser";
const REQUEST: &str = "org.freedesktop.portal.Request";
const METHOD_TIMEOUT: Duration = Duration::from_secs(5);

fn failed(error: FileDialogError) -> FileDialogResult {
    FileDialogResult::Failed(error)
}
fn cancelled(cancel: &Receiver<()>) -> bool {
    cancel.is_closed() || cancel.try_recv().is_ok()
}

async fn setup() -> Result<(Connection, String, u32), FileDialogError> {
    let connection = zbus::connection::Builder::session()
        .map_err(|_| FileDialogError::Unsupported)?
        .method_timeout(METHOD_TIMEOUT)
        .max_queued(8)
        .build()
        .await
        .map_err(|_| FileDialogError::Unsupported)?;
    let (owner, version) = discover(&connection).await?;
    Ok((connection, owner, version))
}

async fn read_version(connection: &Connection, destination: &str) -> Result<u32, FileDialogError> {
    let reply = connection
        .call_method(
            Some(destination),
            DESKTOP,
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &(CHOOSER, "version"),
        )
        .await
        .map_err(|_| FileDialogError::Unsupported)?;
    let value: zbus::zvariant::OwnedValue = reply
        .body()
        .deserialize()
        .map_err(|_| FileDialogError::Unsupported)?;
    u32::try_from(value).map_err(|_| FileDialogError::Unsupported)
}

async fn discover(connection: &Connection) -> Result<(String, u32), FileDialogError> {
    // Activate through the well-known name, then pin both capabilities and
    // requests to its unique owner. A service restart between these operations
    // must not pair an old version with a different service's behavior.
    read_version(connection, DESTINATION).await?;
    let reply = connection
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "GetNameOwner",
            &DESTINATION,
        )
        .await
        .map_err(|_| FileDialogError::Unsupported)?;
    let owner: zbus::names::OwnedUniqueName = reply
        .body()
        .deserialize()
        .map_err(|_| FileDialogError::Unsupported)?;
    let version = read_version(connection, owner.as_str()).await?;
    Ok((owner.to_string(), version))
}

/// Probe actual portal availability/version; no GUI is presented. The result is
/// a snapshot, not a guarantee that a later request will succeed.
pub async fn version() -> Result<u32, FileDialogError> {
    future::or(
        async {
            let (connection, _, version) = setup().await?;
            let _ = connection.close().await;
            if version == 0 {
                Err(FileDialogError::Unsupported)
            } else {
                Ok(version)
            }
        },
        async {
            async_io::Timer::after(METHOD_TIMEOUT).await;
            Err(FileDialogError::Unsupported)
        },
    )
    .await
}

/// Run one window-owned request. The caller must retain the exported parent for
/// native parenting and keep polling this future through cancellation cleanup.
/// Sending cancellation OR dropping its sole sender initiates Request.Close.
/// Acknowledged closure/already-removed requests return Closed; an unacknowledged
/// cleanup failure returns NativeFailure. Dropping this future is not a
/// replacement for awaiting physical portal cancellation.
/// The request uses a dedicated bus connection and closes it on every outcome.
pub async fn choose(
    config: FileDialogConfig,
    parent: &str,
    token: &str,
    cancel: Receiver<()>,
) -> FileDialogResult {
    if !valid_request(&config, parent, token) {
        return failed(FileDialogError::InvalidRequest);
    }
    if matches!(&config, FileDialogConfig::Open(config) if config.selection == FileSelection::FilesAndDirectories)
    {
        return failed(FileDialogError::Unsupported);
    }
    if cancelled(&cancel) {
        return failed(FileDialogError::Closed);
    }
    enum Setup {
        Ready(Result<(Connection, String, u32), FileDialogError>),
        Closed,
    }
    let ready = future::or(
        async {
            Setup::Ready(
                future::or(setup(), async {
                    async_io::Timer::after(METHOD_TIMEOUT).await;
                    Err(FileDialogError::Unsupported)
                })
                .await,
            )
        },
        async {
            let _ = cancel.recv().await;
            Setup::Closed
        },
    )
    .await;
    match ready {
        Setup::Closed => failed(FileDialogError::Closed),
        Setup::Ready(Err(error)) => failed(error),
        Setup::Ready(Ok((connection, owner, version))) => {
            choose_on(connection, &owner, version, config, parent, token, cancel).await
        }
    }
}

fn valid_request(config: &FileDialogConfig, parent: &str, token: &str) -> bool {
    let parent_valid = if let Some(xid) = parent.strip_prefix("x11:") {
        !xid.is_empty()
            && xid.len() <= 16
            && xid.bytes().all(|byte| byte.is_ascii_hexdigit())
            && u64::from_str_radix(xid, 16).is_ok_and(|value| value != 0)
    } else if let Some(handle) = parent.strip_prefix("wayland:") {
        !handle.is_empty() && handle.len() <= 4096 && !handle.contains('\0')
    } else {
        false
    };
    config.is_valid()
        && parent_valid
        && !token.is_empty()
        && token.len() <= 80
        && token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn options<'a>(
    config: &'a FileDialogConfig,
    token: &'a str,
    version: u32,
) -> Result<(&'static str, &'a str, HashMap<&'static str, Value<'a>>), FileDialogError> {
    if version == 0 {
        return Err(FileDialogError::Unsupported);
    }
    let mut options = HashMap::from([
        ("handle_token", Value::from(token)),
        ("modal", Value::from(true)),
    ]);
    let (method, title, label, directory) = match config {
        FileDialogConfig::Capabilities => return Err(FileDialogError::InvalidRequest),
        FileDialogConfig::Open(config) => {
            if config.selection == FileSelection::FilesAndDirectories
                || (version < 3 && config.selection == FileSelection::Directories)
            {
                return Err(FileDialogError::Unsupported);
            }
            options.insert("multiple", Value::from(config.multiple));
            if version >= 3 {
                options.insert(
                    "directory",
                    Value::from(config.selection == FileSelection::Directories),
                );
            }
            (
                "OpenFile",
                &config.title,
                &config.accept_label,
                config.directory.as_ref(),
            )
        }
        FileDialogConfig::Save(config) => {
            options.insert("current_name", Value::from(config.suggested_name.as_str()));
            (
                "SaveFile",
                &config.title,
                &config.accept_label,
                Some(&config.directory),
            )
        }
    };
    options.insert("accept_label", Value::from(label.as_str()));
    if let Some(directory) = directory {
        let mut bytes = directory.as_bytes().to_vec();
        bytes.push(0); // Portal ay paths are explicitly NUL terminated.
        options.insert("current_folder", Value::from(bytes));
    }
    Ok((method, title, options))
}

#[derive(Clone, Copy)]
enum CloseStatus {
    Dismissed,
    Gone,
    Failed,
}

impl CloseStatus {
    fn outcome(self) -> FileDialogResult {
        match self {
            Self::Dismissed | Self::Gone => failed(FileDialogError::Closed),
            Self::Failed => failed(FileDialogError::NativeFailure),
        }
    }
}

async fn close(connection: &Connection, owner: &str, path: &str) -> CloseStatus {
    match connection
        .call_method(Some(owner), path, Some(REQUEST), "Close", &())
        .await
    {
        Ok(_) => CloseStatus::Dismissed,
        Err(zbus::Error::MethodError(name, ..))
            if name.as_str() == "org.freedesktop.DBus.Error.UnknownObject" =>
        {
            CloseStatus::Gone
        }
        Err(_) => CloseStatus::Failed,
    }
}

// Kept separate from session-bus discovery to exercise the actual D-Bus messages
// against a local peer on macOS/Linux, including responses before method replies.
async fn choose_on(
    connection: Connection,
    owner: &str,
    version: u32,
    config: FileDialogConfig,
    parent: &str,
    token: &str,
    cancel: Receiver<()>,
) -> FileDialogResult {
    let result = execute(&connection, owner, version, &config, parent, token, &cancel).await;
    let _ = connection.close().await;
    result
}

async fn execute(
    connection: &Connection,
    owner: &str,
    version: u32,
    config: &FileDialogConfig,
    parent: &str,
    token: &str,
    cancel: &Receiver<()>,
) -> FileDialogResult {
    if !valid_request(config, parent, token) {
        return failed(FileDialogError::InvalidRequest);
    }
    if cancelled(cancel) {
        return failed(FileDialogError::Closed);
    }
    if matches!(config, FileDialogConfig::Capabilities) {
        return if version == 0 {
            failed(FileDialogError::Unsupported)
        } else {
            FileDialogResult::Capabilities(FileDialogCapabilities {
                files: FileSelectionSupport::Multiple,
                directories: if version >= 3 {
                    FileSelectionSupport::Multiple
                } else {
                    FileSelectionSupport::Unsupported
                },
                files_and_directories: FileSelectionSupport::Unsupported,
                save: true,
            })
        };
    }
    let (method, title, options) = match options(config, token, version) {
        Ok(options) => options,
        Err(error) => return failed(error),
    };
    let Some(sender) = connection.unique_name() else {
        return failed(FileDialogError::NativeFailure);
    };
    let namespace = format!(
        "/org/freedesktop/portal/desktop/request/{}/",
        sender.as_str().trim_start_matches(':').replace('.', "_")
    );
    let predicted = format!("{namespace}{token}");
    let rule = (|| -> zbus::Result<_> {
        Ok(MatchRule::builder()
            .msg_type(Type::Signal)
            .sender(owner)?
            .interface(REQUEST)?
            .member("Response")?
            .path_namespace(namespace.trim_end_matches('/'))?
            .build())
    })();
    let rule = match rule {
        Ok(rule) => rule,
        Err(_) => return failed(FileDialogError::NativeFailure),
    };
    let mut responses = match MessageStream::for_match_rule(rule, connection, Some(8)).await {
        Ok(stream) => stream,
        Err(_) => return failed(FileDialogError::NativeFailure),
    };
    // On a real bus, losing the portal does not disconnect our bus connection.
    // Subscribe before issuing OpenFile so service death cannot strand a picker.
    let mut owner_events = if connection.is_bus() {
        match watch_owner(connection, owner).await {
            Ok(stream) => Some(stream),
            Err(_) => return failed(FileDialogError::NativeFailure),
        }
    } else {
        None
    };
    if cancelled(cancel) {
        return failed(FileDialogError::Closed);
    }
    let call = async {
        connection
            .call_method(
                Some(owner),
                DESKTOP,
                Some(CHOOSER),
                method,
                &(parent, title, options),
            )
            .await?
            .body()
            .deserialize::<OwnedObjectPath>()
    };
    let mut call = std::pin::pin!(call);
    enum Started {
        Reply(zbus::Result<OwnedObjectPath>),
        Closed,
    }
    let started = future::or(async { Started::Reply(call.as_mut().await) }, async {
        let _ = cancel.recv().await;
        Started::Closed
    })
    .await;
    let valid_path = |path: &str| path.starts_with(&namespace) && path.len() > namespace.len();
    let actual = match started {
        Started::Closed => {
            // Close the predicted handle promptly. The method may not have
            // created it yet, or may return a legacy alternative handle. Keep
            // polling the method, then close that exact handle as well.
            let first = close(connection, owner, &predicted).await;
            return match call.await {
                Ok(actual) if valid_path(actual.as_str()) => {
                    if matches!(first, CloseStatus::Dismissed) && actual.as_str() == predicted {
                        first.outcome()
                    } else {
                        close(connection, owner, actual.as_str()).await.outcome()
                    }
                }
                Err(_) if matches!(first, CloseStatus::Dismissed) => first.outcome(),
                Ok(_) | Err(_) => failed(FileDialogError::NativeFailure),
            };
        }

        Started::Reply(Err(_)) => {
            let _ = close(connection, owner, &predicted).await;
            return failed(FileDialogError::NativeFailure);
        }
        Started::Reply(Ok(path)) if valid_path(path.as_str()) => path,
        Started::Reply(Ok(_)) => {
            // Never close another connection/request namespace on a bad reply.
            let _ = close(connection, owner, &predicted).await;
            return failed(FileDialogError::NativeFailure);
        }
    };
    enum Completed {
        Response(Option<zbus::Result<zbus::Message>>),
        Closed,
        ServiceGone,
    }
    loop {
        if cancelled(cancel) {
            return close(connection, owner, actual.as_str()).await.outcome();
        }
        let completed = future::or(
            async { Completed::Response(responses.next().await) },
            future::or(
                async {
                    let _ = cancel.recv().await;
                    Completed::Closed
                },
                async {
                    if let Some(stream) = &mut owner_events {
                        let _ = stream.next().await;
                        Completed::ServiceGone
                    } else {
                        future::pending().await
                    }
                },
            ),
        )
        .await;
        match completed {
            Completed::ServiceGone => return failed(FileDialogError::NativeFailure),
            Completed::Closed => {
                return close(connection, owner, actual.as_str()).await.outcome();
            }
            Completed::Response(Some(Ok(message))) => {
                if message
                    .header()
                    .path()
                    .is_some_and(|path| path.as_str() == actual.as_str())
                {
                    return response::decode(&message, config);
                }
            }
            Completed::Response(Some(Err(_)) | None) => {
                let _ = close(connection, owner, actual.as_str()).await;
                return failed(FileDialogError::NativeFailure);
            }
        }
    }
}

async fn watch_owner(connection: &Connection, owner: &str) -> zbus::Result<MessageStream> {
    let rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .sender("org.freedesktop.DBus")?
        .interface("org.freedesktop.DBus")?
        .path("/org/freedesktop/DBus")?
        .member("NameOwnerChanged")?
        .arg(0, owner)?
        .arg(2, "")?
        .build();
    MessageStream::for_match_rule(rule, connection, Some(2)).await
}

#[cfg(test)]
mod tests;

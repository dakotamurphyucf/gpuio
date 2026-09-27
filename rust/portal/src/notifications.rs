//! Freedesktop notification transport. One daemon owner per connection; never
//! retarget native IDs after owner loss. Application lifetime policy stays native.
use futures_lite::{StreamExt, future};
use gpuio_protocol::{
    desktop::Identity,
    notification::{Capabilities, ClosedReason, Content, Error, Receipt, Sound, valid_action_id},
};
use std::{collections::HashMap, time::Duration};
use zbus::{Connection, MatchRule, Message, MessageStream, message::Type, zvariant::Value};

const SERVICE: &str = "org.freedesktop.Notifications";
const PATH: &str = "/org/freedesktop/Notifications";
const BUS: &str = "org.freedesktop.DBus";
const BUS_PATH: &str = "/org/freedesktop/DBus";
const TIMEOUT: Duration = Duration::from_secs(5);
const MAX_SIGNAL_BYTES: usize = 512;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Signal {
    Activated(u32),
    Action {
        native_id: u32,
        receipt: i64,
        revision: i64,
        id: String,
    },
    Closed(u32, ClosedReason),
}

pub struct Client {
    connection: Connection,
    owner: String,
    capabilities: Capabilities,
    markup: bool,
}

/// Drain concurrently with method calls. A full zbus subscription backpressures
/// its connection, so awaiting Notify without polling these streams can prevent
/// the method reply from being read. Native state owns event admission/ordering.
pub struct Events {
    owner: String,
    notifications: MessageStream,
    owner_changes: MessageStream,
}

fn map_error(error: zbus::Error) -> Error {
    if let zbus::Error::MethodError(name, _, _) = error {
        match name.as_str() {
            "org.freedesktop.DBus.Error.AccessDenied" | "org.freedesktop.DBus.Error.AuthFailed" => {
                Error::Denied
            }
            "org.freedesktop.DBus.Error.NameHasNoOwner"
            | "org.freedesktop.DBus.Error.ServiceUnknown" => Error::Unavailable,
            "org.freedesktop.DBus.Error.UnknownMethod"
            | "org.freedesktop.DBus.Error.UnknownInterface" => Error::Unsupported,
            _ => Error::NativeFailure,
        }
    } else {
        Error::NativeFailure
    }
}
async fn owner(connection: &Connection) -> Result<String, Error> {
    let reply = connection
        .call_method(Some(BUS), BUS_PATH, Some(BUS), "GetNameOwner", &SERVICE)
        .await
        .map_err(map_error)?;
    if reply.body().len() > 512 {
        return Err(Error::NativeFailure);
    }
    let body = reply.body();
    let owner: &str = body.deserialize().map_err(|_| Error::NativeFailure)?;
    if !owner.starts_with(':') || owner.len() > 255 {
        return Err(Error::NativeFailure);
    }
    Ok(owner.into())
}
fn capabilities(names: &[&str]) -> Result<(Capabilities, bool), Error> {
    if names.len() > 64
        || names.iter().any(|name| {
            name.len() > 128
                || name.is_empty()
                || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
        })
    {
        return Err(Error::NativeFailure);
    }
    let has = |name| names.contains(&name);
    Ok((
        Capabilities {
            body: has("body"),
            actions: has("actions"),
            activation: has("actions"),
            replacement: true,
            dismissal: true,
            permission_request: false,
            sound: has("sound"),
        },
        has("body-markup"),
    ))
}
fn escape_markup(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn action_key(receipt: &Receipt, revision: i64, id: &str) -> String {
    format!("gpuio/{}/{revision}/{id}", receipt.id)
}
fn parse_action(native_id: u32, key: &str) -> Option<Signal> {
    let mut fields = key.strip_prefix("gpuio/")?.split('/');
    let receipt = fields.next()?;
    let revision = fields.next()?;
    let id = fields.next()?;
    let receipt_id: i64 = receipt.parse().ok()?;
    let revision_id: i64 = revision.parse().ok()?;
    if fields.next().is_some()
        || receipt_id <= 0
        || revision_id <= 0
        || receipt_id.to_string() != receipt
        || revision_id.to_string() != revision
        || !valid_action_id(id)
    {
        return None;
    }
    Some(Signal::Action {
        native_id,
        receipt: receipt_id,
        revision: revision_id,
        id: id.into(),
    })
}

// zbus's method timeout starts after writing the request. Bound the whole
// operation as well, including a backpressured socket write, so native shutdown
// cannot wait indefinitely for a daemon/bus that has stopped reading.
async fn bounded<T>(work: impl std::future::Future<Output = Result<T, Error>>) -> Result<T, Error> {
    future::or(work, async {
        async_io::Timer::after(TIMEOUT).await;
        Err(Error::NativeFailure)
    })
    .await
}

impl Client {
    /// Session setup has a bounded deadline. Does not start or replace a daemon.
    /// Callers must keep draining signals and close the connection on failure.
    pub async fn connect() -> Result<(Self, Events), Error> {
        future::or(
            async {
                let connection = zbus::connection::Builder::session()
                    .map_err(|_| Error::Unavailable)?
                    .method_timeout(TIMEOUT)
                    .max_queued(64)
                    .build()
                    .await
                    .map_err(|_| Error::Unavailable)?;
                Self::from_connection(connection).await
            },
            async {
                async_io::Timer::after(TIMEOUT).await;
                Err(Error::Unavailable)
            },
        )
        .await
    }
    async fn from_connection(connection: Connection) -> Result<(Self, Events), Error> {
        // Install ownership observation before resolving the well-known name.
        let rule = MatchRule::builder()
            .msg_type(Type::Signal)
            .sender(BUS)
            .map_err(map_error)?
            .path(BUS_PATH)
            .map_err(map_error)?
            .interface(BUS)
            .map_err(map_error)?
            .member("NameOwnerChanged")
            .map_err(map_error)?
            .arg(0, SERVICE)
            .map_err(map_error)?
            .build();
        let owner_changes = MessageStream::for_match_rule(rule, &connection, Some(8))
            .await
            .map_err(map_error)?;
        let owner = owner(&connection).await?;
        let rule = MatchRule::builder()
            .msg_type(Type::Signal)
            .sender(owner.as_str())
            .map_err(map_error)?
            .path(PATH)
            .map_err(map_error)?
            .interface(SERVICE)
            .map_err(map_error)?
            .build();
        let notifications = MessageStream::for_match_rule(rule, &connection, Some(64))
            .await
            .map_err(map_error)?;
        let reply = connection
            .call_method(
                Some(owner.as_str()),
                PATH,
                Some(SERVICE),
                "GetCapabilities",
                &(),
            )
            .await
            .map_err(map_error)?;
        if reply.body().len() > 8192 {
            return Err(Error::NativeFailure);
        }
        let body = reply.body();
        let names: Vec<&str> = body.deserialize().map_err(|_| Error::NativeFailure)?;
        let (capabilities, markup) = capabilities(&names)?;
        if self::owner(&connection).await? != owner {
            return Err(Error::Unavailable);
        }
        let events = Events {
            owner: owner.clone(),
            notifications,
            owner_changes,
        };
        Ok((
            Self {
                connection,
                owner,
                capabilities,
                markup,
            },
            events,
        ))
    }
    pub fn capabilities(&self) -> Capabilities {
        self.capabilities
    }
    pub async fn show(
        &self,
        identity: &Identity,
        receipt: &Receipt,
        revision: i64,
        replaces: u32,
        content: &Content,
    ) -> Result<u32, Error> {
        bounded(self.show_inner(identity, receipt, revision, replaces, content)).await
    }
    async fn show_inner(
        &self,
        identity: &Identity,
        receipt: &Receipt,
        revision: i64,
        replaces: u32,
        content: &Content,
    ) -> Result<u32, Error> {
        if !identity.is_valid() || !receipt.is_valid() || revision <= 0 || !content.is_valid() {
            return Err(Error::InvalidRequest);
        }
        if (!self.capabilities.body && !content.body.is_empty())
            || (!self.capabilities.actions && !content.actions.is_empty())
            || (content.sound == Sound::Default && !self.capabilities.sound)
        {
            return Err(Error::Unsupported);
        }
        let mut actions = Vec::with_capacity(2 + content.actions.len() * 2);
        if self.capabilities.activation {
            actions.extend(["default".to_owned(), "Open".to_owned()]);
        }
        for action in &content.actions {
            actions.extend([
                action_key(receipt, revision, &action.id),
                action.label.clone(),
            ]);
        }
        let body = if self.markup {
            escape_markup(&content.body)
        } else {
            content.body.clone()
        };
        let mut hints = HashMap::new();
        hints.insert("desktop-entry", Value::from(identity.identifier.as_str()));
        hints.insert(
            "suppress-sound",
            Value::from(content.sound == Sound::Silent),
        );
        hints.insert("resident", Value::from(false));
        let reply = self
            .connection
            .call_method(
                Some(self.owner.as_str()),
                PATH,
                Some(SERVICE),
                "Notify",
                &(
                    &identity.name,
                    replaces,
                    "",
                    &content.title,
                    &body,
                    actions,
                    hints,
                    -1i32,
                ),
            )
            .await
            .map_err(map_error)?;
        if reply.body().len() != 4 {
            return Err(Error::NativeFailure);
        }
        let native_id: u32 = reply
            .body()
            .deserialize()
            .map_err(|_| Error::NativeFailure)?;
        if native_id == 0 || (replaces != 0 && native_id != replaces) {
            return Err(Error::NativeFailure);
        }
        Ok(native_id)
    }
    pub async fn dismiss(&self, native_id: u32) -> Result<(), Error> {
        bounded(self.dismiss_inner(native_id)).await
    }
    async fn dismiss_inner(&self, native_id: u32) -> Result<(), Error> {
        if native_id == 0 {
            return Err(Error::InvalidRequest);
        }
        let reply = self
            .connection
            .call_method(
                Some(self.owner.as_str()),
                PATH,
                Some(SERVICE),
                "CloseNotification",
                &native_id,
            )
            .await
            .map_err(map_error)?;
        if !reply.body().is_empty() {
            return Err(Error::NativeFailure);
        }
        Ok(())
    }

    pub async fn close(self) {
        let _ = bounded(async { self.connection.close().await.map_err(map_error) }).await;
    }
}

impl Events {
    /// Subscriptions use bounded backpressure. Disconnect and owner loss are
    /// errors, never silently interpreted as an empty input queue.
    pub async fn next_signal(&mut self) -> Result<Signal, Error> {
        enum Incoming {
            Notification(Option<Result<Message, zbus::Error>>),
            Owner(Option<Result<Message, zbus::Error>>),
        }
        let mut drained = 0;
        loop {
            // Foreign-app traffic must not monopolize the worker and starve
            // method deadlines/cancellation while every receive is ready.
            if drained == 32 {
                future::yield_now().await;
                drained = 0;
            }
            drained += 1;
            let incoming = future::or(
                async { Incoming::Owner(self.owner_changes.next().await) },
                async { Incoming::Notification(self.notifications.next().await) },
            )
            .await;
            match incoming {
                Incoming::Owner(Some(Ok(message))) => {
                    if message.body().len() > 1024 {
                        return Err(Error::NativeFailure);
                    }
                    let body = message.body();
                    let (name, before, after): (&str, &str, &str) =
                        body.deserialize().map_err(|_| Error::NativeFailure)?;
                    if name == SERVICE && before == self.owner && after != self.owner {
                        return Err(Error::Unavailable);
                    }
                }
                Incoming::Notification(Some(Ok(message))) => {
                    // Other applications share this daemon. Ignore oversized or
                    // unrelated action payloads; never retain their text.
                    if message.body().len() > MAX_SIGNAL_BYTES {
                        continue;
                    }
                    let header = message.header();
                    if header.sender().map(|name| name.as_str()) != Some(self.owner.as_str()) {
                        continue;
                    }
                    match header.member().map(|name| name.as_str()) {
                        Some("ActionInvoked") => {
                            let body = message.body();
                            let (native_id, key): (u32, &str) =
                                body.deserialize().map_err(|_| Error::NativeFailure)?;
                            if native_id == 0 {
                                continue;
                            }
                            if key == "default" {
                                return Ok(Signal::Activated(native_id));
                            }
                            if let Some(signal) = parse_action(native_id, key) {
                                return Ok(signal);
                            }
                        }
                        Some("NotificationClosed") => {
                            let (native_id, reason): (u32, u32) = message
                                .body()
                                .deserialize()
                                .map_err(|_| Error::NativeFailure)?;
                            if native_id != 0 {
                                return Ok(Signal::Closed(
                                    native_id,
                                    match reason {
                                        1 => ClosedReason::Expired,
                                        2 => ClosedReason::User,
                                        _ => ClosedReason::Platform,
                                    },
                                ));
                            }
                        }
                        _ => (),
                    }
                }
                Incoming::Owner(None | Some(Err(_)))
                | Incoming::Notification(None | Some(Err(_))) => return Err(Error::Unavailable),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsupported_content_and_plaintext_policy_are_explicit() {
        let (caps, markup) = capabilities(&["body", "actions", "body-markup", "sound"]).unwrap();
        assert!(caps.body && caps.actions && caps.activation && caps.sound && markup);
        assert!(caps.replacement && caps.dismissal && !caps.permission_request);
        assert!(!capabilities(&[]).unwrap().0.actions);
        assert!(capabilities(&["invalid cap"]).is_err());
        assert!(capabilities(&["body"; 65]).is_err());
        assert_eq!(
            escape_markup("a <b>&\"'🦀"),
            "a &lt;b&gt;&amp;&quot;&apos;🦀"
        );
    }
    #[test]
    fn action_transport_keys_carry_both_lifetime_and_revision() {
        let receipt = Receipt {
            id: 7,
            tag: "build".into(),
        };
        assert_eq!(
            parse_action(42, &action_key(&receipt, 11, "open")),
            Some(Signal::Action {
                native_id: 42,
                receipt: 7,
                revision: 11,
                id: "open".into()
            })
        );
        for key in [
            "open",
            "gpuio/0/1/open",
            "gpuio/1/0/open",
            "gpuio/01/2/open",
            "gpuio/1/2/a/b",
            "gpuio/1/2/",
        ] {
            assert!(parse_action(42, key).is_none());
        }
    }
}

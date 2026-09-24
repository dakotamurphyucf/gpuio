//! Event-driven XDG Settings observation. No display dependency or idle polling.
use futures_lite::{StreamExt, future};
use std::time::Duration;
use zbus::{Connection, MatchRule, MessageStream, message::Type, zvariant::OwnedValue};
const DESTINATION: &str = "org.freedesktop.portal.Desktop";
const PATH: &str = "/org/freedesktop/portal/desktop";
const INTERFACE: &str = "org.freedesktop.portal.Settings";
const NAMESPACE: &str = "org.freedesktop.appearance";
const KEY: &str = "reduced-motion";
const TIMEOUT: Duration = Duration::from_secs(5);

fn decode(value: OwnedValue) -> Option<bool> {
    // The standard maps unrecognized numeric values to no preference. A wrong
    // D-Bus type, unavailable portal or missing key is explicitly unavailable.
    u32::try_from(value).ok().map(|value| value == 1)
}
async fn read(connection: &Connection) -> Option<bool> {
    let reply = connection
        .call_method(
            Some(DESTINATION),
            PATH,
            Some(INTERFACE),
            "ReadOne",
            &(NAMESPACE, KEY),
        )
        .await
        .ok()?;
    decode(reply.body().deserialize().ok()?)
}
async fn streams(connection: &Connection) -> zbus::Result<(MessageStream, MessageStream)> {
    let setting = MatchRule::builder()
        .msg_type(Type::Signal)
        .sender(DESTINATION)?
        .path(PATH)?
        .interface(INTERFACE)?
        .member("SettingChanged")?
        .arg(0, NAMESPACE)?
        .arg(1, KEY)?
        .build();
    let owner = MatchRule::builder()
        .msg_type(Type::Signal)
        .sender("org.freedesktop.DBus")?
        .path("/org/freedesktop/DBus")?
        .interface("org.freedesktop.DBus")?
        .member("NameOwnerChanged")?
        .arg(0, DESTINATION)?
        .build();
    Ok((
        MessageStream::for_match_rule(setting, connection, Some(8)).await?,
        MessageStream::for_match_rule(owner, connection, Some(8)).await?,
    ))
}
async fn observe(
    connection: &Connection,
    changed: &mut impl FnMut(Option<bool>),
) -> zbus::Result<()> {
    // Subscribe before reading. Signals trigger a fresh read rather than applying
    // their payload: a queued signal from before ReadOne must not revert a newer
    // snapshot. Name ownership changes also refresh the current service's state.
    let (mut settings, mut owners) = future::or(streams(connection), async {
        async_io::Timer::after(TIMEOUT).await;
        Err(zbus::Error::Failure(
            "settings subscription timed out".into(),
        ))
    })
    .await?;
    loop {
        changed(
            future::or(read(connection), async {
                async_io::Timer::after(TIMEOUT).await;
                None
            })
            .await,
        );
        let next = future::or(settings.next(), owners.next()).await;
        match next {
            Some(Ok(_)) => (),
            Some(Err(error)) => return Err(error),
            None => return Ok(()),
        }
    }
}

/// Follow the standardized reduced-motion setting and portal service restarts.
/// Updates are snapshots; None means unavailable. Dropping this future releases
/// the private connection/subscriptions. Startup is bounded and never blocks GPUI.
/// A missing/disconnected session bus falls back until the next app launch.
pub async fn watch_motion(mut changed: impl FnMut(Option<bool>)) {
    let connection = future::or(
        async {
            zbus::connection::Builder::session()?
                .method_timeout(TIMEOUT)
                .max_queued(8)
                .build()
                .await
        },
        async {
            async_io::Timer::after(TIMEOUT).await;
            Err(zbus::Error::Failure("session bus timed out".into()))
        },
    )
    .await;
    if let Ok(connection) = connection {
        // ReadOne also needs a whole-call deadline for an unresponsive service.
        let _ = observe(&connection, &mut changed).await;
        let _ = connection.close().await;
    }
    changed(None);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn standardized_value_and_unknown_fallback() {
        assert_eq!(decode(OwnedValue::from(0_u32)), Some(false));
        assert_eq!(decode(OwnedValue::from(1_u32)), Some(true));
        assert_eq!(decode(OwnedValue::from(2_u32)), Some(false));
        assert_eq!(decode(OwnedValue::from(u32::MAX)), Some(false));
        assert_eq!(decode(OwnedValue::from(true)), None);
    }
}

#[cfg(test)]
mod observation_tests {
    use super::*;
    use std::os::unix::net::UnixStream;
    use zbus::{Message, connection::Builder};

    async fn peer() -> (MessageStream, Connection, Connection) {
        let (a, b) = UnixStream::pair().unwrap();
        let guid = zbus::Guid::generate();
        let server = Builder::authenticated_socket(async_io::Async::new(a).unwrap(), guid.clone())
            .unwrap()
            .p2p()
            .unique_name(":1.10")
            .unwrap()
            .build_message_stream();
        let client = Builder::authenticated_socket(async_io::Async::new(b).unwrap(), guid)
            .unwrap()
            .p2p()
            .unique_name(":1.20")
            .unwrap()
            .method_timeout(Duration::from_millis(100))
            .build();
        let (stream, client) = future::zip(server, client).await;
        let stream = stream.unwrap();
        let server = Connection::from(&stream);
        (stream, server, client.unwrap())
    }
    async fn read_call(stream: &mut MessageStream) -> Message {
        let message = stream.next().await.unwrap().unwrap();
        assert_eq!(message.header().member().unwrap().as_str(), "ReadOne");
        assert_eq!(
            message.body().deserialize::<(&str, &str)>().unwrap(),
            (NAMESPACE, KEY)
        );
        message
    }
    async fn signal(server: &Connection, value: u32) {
        server
            .emit_signal(
                None::<&str>,
                PATH,
                INTERFACE,
                "SettingChanged",
                &(NAMESPACE, KEY, OwnedValue::from(value)),
            )
            .await
            .unwrap();
    }
    #[test]
    fn live_settings_restart_stale_payload_and_unavailable_service() {
        future::block_on(future::or(
            async {
                let (mut stream, server, client) = peer().await;
                let (sender, updates) = async_channel::bounded(8);
                let observation = async {
                    observe(&client, &mut |value| {
                        sender.try_send(value).unwrap();
                    })
                    .await
                    .unwrap();
                    panic!("observer ended before its owner cancelled it");
                };
                let scenario = async {
                    let call = read_call(&mut stream).await;
                    server
                        .reply(&call.header(), &OwnedValue::from(0_u32))
                        .await
                        .unwrap();
                    assert_eq!(updates.recv().await.unwrap(), Some(false));
                    signal(&server, 0).await; // old payload; current ReadOne is authoritative.
                    let call = read_call(&mut stream).await;
                    server
                        .reply(&call.header(), &OwnedValue::from(1_u32))
                        .await
                        .unwrap();
                    assert_eq!(updates.recv().await.unwrap(), Some(true));
                    let owner = Message::signal(
                        "/org/freedesktop/DBus",
                        "org.freedesktop.DBus",
                        "NameOwnerChanged",
                    )
                    .unwrap()
                    .sender("org.freedesktop.DBus")
                    .unwrap()
                    .build(&(DESTINATION, ":1.10", ":1.11"))
                    .unwrap();
                    server.send(&owner).await.unwrap();
                    let call = read_call(&mut stream).await;
                    server
                        .reply(&call.header(), &OwnedValue::from(0_u32))
                        .await
                        .unwrap();
                    assert_eq!(updates.recv().await.unwrap(), Some(false));
                    signal(&server, 1).await;
                    let call = read_call(&mut stream).await;
                    server
                        .reply_error(
                            &call.header(),
                            "org.freedesktop.portal.Error.NotFound",
                            &"no setting",
                        )
                        .await
                        .unwrap();
                    assert_eq!(updates.recv().await.unwrap(), None);
                    signal(&server, 1).await;
                    let _unanswered = read_call(&mut stream).await;
                    assert_eq!(
                        updates.recv().await.unwrap(),
                        None,
                        "unresponsive service has a bounded call deadline"
                    );
                };
                future::or(scenario, observation).await; // Drops observer and both subscriptions.
                client.close().await.unwrap();
            },
            async {
                async_io::Timer::after(Duration::from_secs(5)).await;
                panic!("motion observer test timed out");
            },
        ));
    }
}

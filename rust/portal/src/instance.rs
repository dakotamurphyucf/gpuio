//! Session-bus launch arbitration. No display or OCaml callbacks. A primary owns
//! its name until the server is closed; a secondary returns only after admission.
use async_channel::Receiver;
use futures_lite::{StreamExt, future};
use gpuio_protocol::desktop::{
    Error, MAX_LINK_BATCH_BYTES, MAX_LINK_BYTES, MAX_LINKS, valid_identifier,
};
use std::{sync::Arc, time::Duration};
use zbus::{Connection, Message, MessageStream, message::Type};

const PATH: &str = "/org/gpuio/Application1";
const INTERFACE: &str = "org.gpuio.Application1";
const BUS: &str = "org.freedesktop.DBus";
const BUS_PATH: &str = "/org/freedesktop/DBus";
const TIMEOUT: Duration = Duration::from_secs(5);

/// Admission must be synchronous, atomic, bounded and free of application code.
/// An empty forwarded batch requests the existing application's reopen policy.
pub type Admit = Arc<dyn Fn(Vec<String>) -> Result<(), Error> + Send + Sync>;

pub enum Launch {
    Primary(Box<Server>),
    Forwarded,
}

pub struct Server {
    connection: Connection,
    stream: MessageStream,
    admit: Admit,
    identifier: String,
}

fn valid_links(links: &[impl AsRef<str>]) -> bool {
    links.len() <= MAX_LINKS
        && links.iter().all(|s| {
            let s = s.as_ref();
            s.len() <= MAX_LINK_BYTES && !s.contains('\0')
        })
        && links.iter().map(|s| s.as_ref().len()).sum::<usize>() <= MAX_LINK_BATCH_BYTES
}

fn map_error(error: zbus::Error) -> Error {
    if let zbus::Error::MethodError(name, _, _) = error {
        match name.as_str() {
            "org.gpuio.Application1.Error.InvalidRequest" => Error::InvalidRequest,
            "org.gpuio.Application1.Error.Busy" => Error::Busy,
            "org.gpuio.Application1.Error.Closed" => Error::Closed,
            "org.freedesktop.DBus.Error.AccessDenied" => Error::Denied,
            "org.freedesktop.DBus.Error.NameHasNoOwner"
            | "org.freedesktop.DBus.Error.ServiceUnknown" => Error::Unavailable,
            _ => Error::NativeFailure,
        }
    } else {
        // A failed/expired reply can follow successful admission. Do not retry.
        Error::NativeFailure
    }
}

/// Claim once before constructing the UI. Startup input precedes any forwarded
/// input. A missing bus is Unavailable; uncertain forwarding is NativeFailure.
/// No implicit fallback starts a second application after a forwarding failure.
pub async fn launch(identifier: &str, links: Vec<String>, admit: Admit) -> Result<Launch, Error> {
    if !valid_identifier(identifier) || !valid_links(&links) {
        return Err(Error::InvalidRequest);
    }
    let connection = future::or(
        async {
            zbus::connection::Builder::session()
                .map_err(|_| Error::Unavailable)?
                .method_timeout(TIMEOUT)
                .max_queued(8)
                .build()
                .await
                .map_err(|_| Error::Unavailable)
        },
        async {
            async_io::Timer::after(TIMEOUT).await;
            Err(Error::Unavailable)
        },
    )
    .await?;
    future::or(
        claim(MessageStream::from(&connection), identifier, links, admit),
        async {
            async_io::Timer::after(TIMEOUT).await;
            Err(Error::NativeFailure)
        },
    )
    .await
}

async fn claim(
    stream: MessageStream,
    identifier: &str,
    links: Vec<String>,
    admit: Admit,
) -> Result<Launch, Error> {
    let connection = Connection::from(&stream);
    // Subscribe before claiming. Use raw RequestName because this server owns
    // the message stream, not zbus's object server. 4 = DO_NOT_QUEUE, with neither
    // replacement flag. Ownership is atomic in the bus, not a check-then-claim.
    let reply = connection
        .call_method(
            Some(BUS),
            BUS_PATH,
            Some(BUS),
            "RequestName",
            &(identifier, 4u32),
        )
        .await
        .map_err(map_error)?;
    let result: u32 = reply
        .body()
        .deserialize()
        .map_err(|_| Error::NativeFailure)?;
    match result {
        1 => {
            if !links.is_empty() {
                admit(links)?;
            }
            Ok(Launch::Primary(Box::new(Server {
                connection,
                stream,
                admit,
                identifier: identifier.to_owned(),
            })))
        }
        3 => {
            let reply = connection
                .call_method(Some(BUS), BUS_PATH, Some(BUS), "GetNameOwner", &identifier)
                .await
                .map_err(map_error)?;
            let owner: zbus::names::OwnedUniqueName = reply
                .body()
                .deserialize()
                .map_err(|_| Error::NativeFailure)?;
            forward(&connection, owner.as_str(), &links).await?;
            Ok(Launch::Forwarded)
        }
        _ => Err(Error::NativeFailure),
    }
}

async fn forward(connection: &Connection, owner: &str, links: &[String]) -> Result<(), Error> {
    connection
        .call_method(Some(owner), PATH, Some(INTERFACE), "OpenLinks", &(links,))
        .await
        .map_err(map_error)?
        .body()
        .deserialize::<()>()
        .map_err(|_| Error::NativeFailure)
}

fn decode_links(message: &Message) -> Result<Vec<String>, Error> {
    let body = message.body();
    // Bound allocation before decoding the array. Borrow strings until every
    // length/count constraint has passed; malformed links remain application data.
    if body.len() > MAX_LINK_BATCH_BYTES + 1024 {
        return Err(Error::InvalidRequest);
    }
    let (links,): (Vec<&str>,) = body.deserialize().map_err(|_| Error::InvalidRequest)?;
    if !valid_links(&links) {
        return Err(Error::InvalidRequest);
    }
    Ok(links.into_iter().map(str::to_owned).collect())
}

impl Server {
    async fn reply(&self, message: &Message) -> Result<(), Error> {
        let header = message.header();
        if header.path().map(|s| s.as_str()) != Some(PATH)
            || header.interface().map(|s| s.as_str()) != Some(INTERFACE)
            || header.member().map(|s| s.as_str()) != Some("OpenLinks")
        {
            return self
                .connection
                .reply_error(
                    &header,
                    "org.freedesktop.DBus.Error.UnknownMethod",
                    &"Unknown method",
                )
                .await
                .map_err(map_error);
        }
        match decode_links(message).and_then(|links| (self.admit)(links)) {
            Ok(()) => self.connection.reply(&header, &()).await.map_err(map_error),
            Err(error) => {
                let name = match error {
                    Error::InvalidRequest => "org.gpuio.Application1.Error.InvalidRequest",
                    Error::Busy => "org.gpuio.Application1.Error.Busy",
                    Error::Closed => "org.gpuio.Application1.Error.Closed",
                    _ => "org.gpuio.Application1.Error.NativeFailure",
                };
                self.connection
                    .reply_error(&header, name, &"Launch was not admitted")
                    .await
                    .map_err(map_error)
            }
        }
    }

    /// Await until explicit cancellation or bus loss. Cancellation closes the
    /// connection/name before returning. Unexpected loss is an error for the
    /// native owner to surface; it must not silently restart/replay launches.
    pub async fn serve(mut self, cancel: Receiver<()>) -> Result<(), Error> {
        let result = future::or(
            async {
                loop {
                    let message = self
                        .stream
                        .next()
                        .await
                        .ok_or(Error::Unavailable)?
                        .map_err(|_| Error::Unavailable)?;
                    let header = message.header();
                    match message.message_type() {
                        Type::MethodCall => {
                            future::or(self.reply(&message), async {
                                async_io::Timer::after(TIMEOUT).await;
                                Err(Error::NativeFailure)
                            })
                            .await?;
                        }
                        Type::Signal
                            if header.sender().map(|s| s.as_str()) == Some(BUS)
                                && header.interface().map(|s| s.as_str()) == Some(BUS)
                                && header.member().map(|s| s.as_str()) == Some("NameLost")
                                && message.body().deserialize::<&str>().ok()
                                    == Some(self.identifier.as_str()) =>
                        {
                            return Err(Error::Unavailable);
                        }
                        _ => (),
                    }
                }
            },
            async {
                let _ = cancel.recv().await;
                Ok(())
            },
        )
        .await;
        // Close is socket shutdown, not a remote round trip.
        let _ = self.connection.close().await;
        result
    }
}

#[cfg(test)]
mod tests;

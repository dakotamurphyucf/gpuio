//! XDG OpenURI file services, independent of GPUI and display libraries.
use crate::request::{setup_interface, transact};
use async_channel::Receiver;
use futures_lite::future;
use gpuio_protocol::{desktop::Error, file_dialog::FileDialogError};
use std::{collections::HashMap, fs::File, os::fd::AsFd, time::Duration};
use zbus::{
    Connection,
    zvariant::{Fd, OwnedObjectPath, Value},
};

const INTERFACE: &str = "org.freedesktop.portal.OpenURI";
const PATH: &str = "/org/freedesktop/portal/desktop";
const TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileOperation {
    Open,
    Reveal,
}

impl FileOperation {
    fn minimum_version(self) -> u32 {
        match self {
            Self::Open => 2,
            Self::Reveal => 3,
        }
    }
    fn method(self) -> &'static str {
        match self {
            Self::Open => "OpenFile",
            Self::Reveal => "OpenDirectory",
        }
    }
}

fn map_error(error: FileDialogError) -> Error {
    match error {
        FileDialogError::Closed => Error::Closed,
        FileDialogError::Unsupported => Error::Unavailable,
        _ => Error::NativeFailure,
    }
}

/// Application-scoped request without a borrowed native window. The owned file
/// descriptor must designate a regular file or directory. Keep polling through
/// cancellation cleanup; dropping the sender cancels a live portal request.
/// Success is the portal's acceptance, not visible presentation in the target app.
pub async fn file(
    file: File,
    operation: FileOperation,
    token: &str,
    cancel: Receiver<()>,
) -> Result<(), Error> {
    if token.is_empty()
        || token.len() > 80
        || !token
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_')
    {
        return Err(Error::InvalidRequest);
    }
    if cancel.is_closed() || cancel.try_recv().is_ok() {
        return Err(Error::Closed);
    }
    let setup = future::or(
        async { setup_interface(INTERFACE).await.map_err(map_error) },
        future::or(
            async {
                async_io::Timer::after(TIMEOUT).await;
                Err(Error::Unavailable)
            },
            async {
                let _ = cancel.recv().await;
                Err(Error::Closed)
            },
        ),
    )
    .await;
    let (connection, owner, version) = setup?;
    let result = file_on(
        &connection,
        &owner,
        version,
        &file,
        operation,
        token,
        &cancel,
    )
    .await;
    let _ = connection.close().await;
    result
}

async fn file_on(
    connection: &Connection,
    owner: &str,
    version: u32,
    file: &File,
    operation: FileOperation,
    token: &str,
    cancel: &Receiver<()>,
) -> Result<(), Error> {
    if version < operation.minimum_version() {
        return Err(Error::Unsupported);
    }
    let mut options = HashMap::from([("handle_token", Value::from(token))]);
    if operation == FileOperation::Open {
        options.insert("writable", Value::from(true));
        if version >= 3 {
            options.insert("ask", Value::from(false));
        }
    }
    let call = async {
        connection
            .call_method(
                Some(owner),
                PATH,
                Some(INTERFACE),
                operation.method(),
                &("", Fd::from(file.as_fd()), options),
            )
            .await?
            .body()
            .deserialize::<OwnedObjectPath>()
    };
    let message = transact(connection, owner, token, cancel, call)
        .await
        .map_err(map_error)?;
    let body = message.body();
    if body.len() > 16_384 {
        return Err(Error::NativeFailure);
    }
    let (code, _fields): (u32, HashMap<&str, Value<'_>>) =
        body.deserialize().map_err(|_| Error::NativeFailure)?;
    match code {
        0 => Ok(()),
        1 => Err(Error::Denied),
        _ => Err(Error::NativeFailure),
    }
}

#[cfg(test)]
mod tests;

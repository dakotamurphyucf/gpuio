use gpuio_protocol::{
    file_dialog::{
        FileDialogConfig, FileDialogError, FileDialogResult, MAX_SELECTED_PATH_BYTES,
        MAX_SELECTED_PATHS,
    },
    file_path::{FilePath, MAX_PATH_BYTES},
    v1::MAX_MESSAGE_BYTES,
};
use std::collections::HashMap;
use zbus::zvariant::Value;

// Decode the local file URI without URL normalization: dot segments, repeated
// slashes, non-UTF-8 percent escapes and the native filename are preserved.
fn file_uri(uri: &str) -> Result<FilePath, FileDialogError> {
    let invalid = FileDialogError::NativeFailure;
    if uri.len() > MAX_PATH_BYTES * 3 + 20 {
        return Err(FileDialogError::LimitExceeded);
    }
    let (scheme, rest) = uri.split_once(':').ok_or(invalid)?;
    if !scheme.eq_ignore_ascii_case("file") {
        return Err(invalid);
    }
    let rest = rest.strip_prefix("//").ok_or(invalid)?;
    let slash = rest.find('/').ok_or(invalid)?;
    let host = &rest[..slash];
    if !host.is_empty() && !host.eq_ignore_ascii_case("localhost") {
        return Err(invalid);
    }
    let raw = &rest.as_bytes()[slash..];
    let mut path = Vec::with_capacity(raw.len().min(MAX_PATH_BYTES));
    let mut index = 0;
    while index < raw.len() {
        let byte = raw[index];
        let decoded = if byte == b'%' {
            let hex = raw.get(index + 1..index + 3).ok_or(invalid)?;
            let high = (hex[0] as char).to_digit(16).ok_or(invalid)?;
            let low = (hex[1] as char).to_digit(16).ok_or(invalid)?;
            index += 3;
            (high * 16 + low) as u8
        } else {
            if byte <= b' ' || matches!(byte, b'?' | b'#' | b'\\' | 127) {
                return Err(invalid);
            }
            index += 1;
            byte
        };
        if path.len() == MAX_PATH_BYTES {
            return Err(FileDialogError::LimitExceeded);
        }
        path.push(decoded);
    }
    FilePath::new(path).map_err(|_| invalid)
}

pub(crate) fn decode(message: &zbus::Message, config: &FileDialogConfig) -> FileDialogResult {
    fn inner(
        message: &zbus::Message,
        config: &FileDialogConfig,
    ) -> Result<Option<Vec<FilePath>>, FileDialogError> {
        // Bound the complete D-Bus body before zvariant materializes containers.
        // String values borrow the message; path allocations have tighter bounds.
        let body = message.body();
        if body.len() > MAX_MESSAGE_BYTES {
            return Err(FileDialogError::LimitExceeded);
        }
        let (code, fields): (u32, HashMap<&str, Value<'_>>) = body
            .deserialize()
            .map_err(|_| FileDialogError::NativeFailure)?;
        match code {
            1 => return Ok(None),
            0 => (),
            _ => return Err(FileDialogError::NativeFailure),
        }
        let Some(Value::Array(uris)) = fields.get("uris") else {
            return Err(FileDialogError::NativeFailure);
        };
        if uris.len() > MAX_SELECTED_PATHS {
            return Err(FileDialogError::LimitExceeded);
        }
        let mut paths = Vec::with_capacity(uris.len());
        let mut bytes = 0;
        for uri in uris.inner() {
            let Value::Str(uri) = uri else {
                return Err(FileDialogError::NativeFailure);
            };
            let path = file_uri(uri.as_str())?;
            bytes += path.as_bytes().len();
            if bytes > MAX_SELECTED_PATH_BYTES {
                return Err(FileDialogError::LimitExceeded);
            }
            paths.push(path);
        }
        if !config.accepts_selection(&paths) {
            return Err(FileDialogError::NativeFailure);
        }
        Ok(Some(paths))
    }
    match inner(message, config) {
        Ok(Some(paths)) => FileDialogResult::Selected(paths),
        Ok(None) => FileDialogResult::Cancelled,
        Err(error) => FileDialogResult::Failed(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpuio_protocol::file_dialog::{FileSelection, OpenFileConfig};

    fn config(multiple: bool) -> FileDialogConfig {
        FileDialogConfig::Open(OpenFileConfig {
            selection: FileSelection::Files,
            multiple,
            title: "Open".into(),
            accept_label: "Choose".into(),
            directory: None,
        })
    }
    fn response(code: u32, uris: &[&str]) -> zbus::Message {
        let fields = HashMap::from([("uris", Value::from(uris.to_vec()))]);
        zbus::Message::signal(
            "/test/request",
            "org.freedesktop.portal.Request",
            "Response",
        )
        .unwrap()
        .build(&(code, fields))
        .unwrap()
    }
    #[test]
    fn preserves_native_bytes_without_normalizing_or_rewriting() {
        assert_eq!(
            file_uri("file:///tmp/%FF/a/../report.sql.s")
                .unwrap()
                .as_bytes(),
            b"/tmp/\xff/a/../report.sql.s"
        );
        assert_eq!(
            file_uri("FILE://localhost/tmp/a%20b%3F%23")
                .unwrap()
                .as_bytes(),
            b"/tmp/a b?#"
        );
        for uri in [
            "https:///tmp/a",
            "file://remote/tmp/a",
            "file://localhost:9/tmp/a",
            "file:///tmp/%00",
            "file:///tmp/%",
            "file:///tmp/%GG",
            "file:///tmp/a?b",
            "file:///tmp/a#b",
            "file:///tmp/a b",
        ] {
            assert!(file_uri(uri).is_err(), "{uri}");
        }
    }
    #[test]
    fn selection_is_all_or_error_and_cancellation_is_distinct() {
        let open = config(true);
        assert!(
            matches!(decode(&response(0, &["file:///a", "file:///b"]), &open), FileDialogResult::Selected(paths) if paths.len() == 2)
        );
        assert_eq!(
            decode(&response(0, &["file:///a", "file://remote/b"]), &open),
            FileDialogResult::Failed(FileDialogError::NativeFailure)
        );
        assert_eq!(
            decode(&response(0, &[]), &open),
            FileDialogResult::Failed(FileDialogError::NativeFailure)
        );
        assert_eq!(
            decode(&response(0, &["file:///a", "file:///b"]), &config(false)),
            FileDialogResult::Failed(FileDialogError::NativeFailure)
        );
        assert_eq!(
            decode(&response(1, &[]), &open),
            FileDialogResult::Cancelled
        );
        for code in [2, 3, u32::MAX] {
            assert_eq!(
                decode(&response(code, &[]), &open),
                FileDialogResult::Failed(FileDialogError::NativeFailure)
            );
        }
        assert_eq!(
            decode(&response(0, &vec!["file:///a"; 129]), &open),
            FileDialogResult::Failed(FileDialogError::LimitExceeded)
        );
        let path = format!("file:///{}", "a".repeat(MAX_PATH_BYTES - 1));
        assert!(matches!(
            decode(&response(0, &vec![path.as_str(); 16]), &open),
            FileDialogResult::Selected(_)
        ));
        assert_eq!(
            decode(&response(0, &vec![path.as_str(); 17]), &open),
            FileDialogResult::Failed(FileDialogError::LimitExceeded)
        );
        assert_eq!(
            file_uri(&format!("{path}a")),
            Err(FileDialogError::LimitExceeded)
        );
    }
}

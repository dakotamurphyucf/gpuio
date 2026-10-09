use super::*;
use crate::desktop::*;

impl Decoder<'_> {
    fn desktop_text(&mut self, limit: usize) -> Result<String, DecodeError> {
        let count = self.count(limit)?;
        let start = self.0.position() as usize;
        let value = std::str::from_utf8(&self.0.get_ref()[start..start + count])
            .map_err(|_| DecodeError::Malformed)?
            .to_owned();
        self.0.set_position((start + count) as u64);
        Ok(value)
    }

    pub(super) fn desktop_request(&mut self) -> Result<Request, DecodeError> {
        let value = match self.tag()? {
            0 => Request::Configure(Identity {
                identifier: self.desktop_text(MAX_IDENTIFIER_BYTES)?,
                name: self.desktop_text(MAX_NAME_BYTES)?,
                schemes: self.list(MAX_SCHEMES, |d| d.desktop_text(MAX_SCHEME_BYTES))?,
            }),
            1 => Request::Capabilities,
            2 => Request::TakeLinks,
            3 => Request::Activate(self.boolean()?),
            4 => Request::RevealFile(self.file_path()?),
            5 => Request::OpenFile(self.file_path()?),
            6 => Request::RegisterScheme(self.desktop_text(MAX_SCHEME_BYTES)?),
            7 => Request::ScrollbarPreference,
            8 => Request::WriteClipboardText(self.desktop_text(MAX_CLIPBOARD_TEXT_BYTES)?),
            _ => return Err(DecodeError::Malformed),
        };
        if value.is_valid() {
            Ok(value)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}

pub fn decode_desktop_request(bytes: &[u8]) -> Result<Request, DecodeError> {
    decode_drag_data(bytes, |decoder| decoder.desktop_request())
}

pub fn decode_desktop_launch(bytes: &[u8]) -> Result<LaunchRequest, DecodeError> {
    decode_drag_data(bytes, |d| {
        let identity = Identity {
            identifier: d.desktop_text(MAX_IDENTIFIER_BYTES)?,
            name: d.desktop_text(MAX_NAME_BYTES)?,
            schemes: d.list(MAX_SCHEMES, |d| d.desktop_text(MAX_SCHEME_BYTES))?,
        };
        let value = LaunchRequest {
            identity,
            links: d.list(MAX_LINKS, |d| d.desktop_text(MAX_LINK_BYTES))?,
        };
        if value.is_valid() {
            Ok(value)
        } else {
            Err(DecodeError::Malformed)
        }
    })
}

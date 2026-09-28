use super::{DecodeError, Decoder, decode_drag_data};
use crate::notification::*;

impl Decoder<'_> {
    fn notification_receipt(&mut self) -> Result<Receipt, DecodeError> {
        Ok(Receipt {
            id: self.int()?,
            tag: self.bounded_text(MAX_TAG_BYTES)?,
        })
    }
    fn notification_content(&mut self) -> Result<Content, DecodeError> {
        Ok(Content {
            title: self.bounded_text(MAX_TITLE_BYTES)?,
            body: self.bounded_text(MAX_BODY_BYTES)?,
            actions: self.list(MAX_ACTIONS, |d| {
                Ok(Action {
                    id: d.bounded_text(MAX_ACTION_ID_BYTES)?,
                    label: d.bounded_text(MAX_LABEL_BYTES)?,
                })
            })?,
            sound: match self.tag()? {
                0 => Sound::Silent,
                1 => Sound::Default,
                _ => return Err(DecodeError::Malformed),
            },
        })
    }
    fn notification_error(&mut self) -> Result<Error, DecodeError> {
        Ok(match self.tag()? {
            0 => Error::InvalidRequest,
            1 => Error::NotReady,
            2 => Error::Unsupported,
            3 => Error::Unavailable,
            4 => Error::Denied,
            5 => Error::Busy,
            6 => Error::Closed,
            7 => Error::Stale,
            8 => Error::NativeFailure,
            _ => return Err(DecodeError::Malformed),
        })
    }
    fn notification_event(&mut self) -> Result<Event, DecodeError> {
        let event = match self.tag()? {
            0 => Event::Activated(self.notification_receipt()?),
            1 => Event::Action(
                self.notification_receipt()?,
                self.bounded_text(MAX_ACTION_ID_BYTES)?,
            ),
            2 => Event::Closed(
                self.notification_receipt()?,
                match self.tag()? {
                    0 => ClosedReason::Expired,
                    1 => ClosedReason::User,
                    2 => ClosedReason::Platform,
                    _ => return Err(DecodeError::Malformed),
                },
            ),
            3 => Event::Failed(self.notification_error()?),
            _ => return Err(DecodeError::Malformed),
        };
        if event.is_valid() {
            Ok(event)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    pub(super) fn notification_request(&mut self) -> Result<Request, DecodeError> {
        let request = match self.tag()? {
            0 => Request::Capabilities,
            1 => Request::Authorization,
            2 => Request::RequestAuthorization,
            3 => Request::Post(
                self.bounded_text(MAX_TAG_BYTES)?,
                self.notification_content()?,
            ),
            4 => Request::Replace(self.notification_receipt()?, self.notification_content()?),
            5 => Request::Dismiss(self.notification_receipt()?),
            6 => Request::TakeEvents,
            7 => Request::Close,
            _ => return Err(DecodeError::Malformed),
        };
        if request.is_valid() {
            Ok(request)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    fn notification_response(&mut self) -> Result<Response, DecodeError> {
        let response = match self.tag()? {
            0 => Response::Capabilities(Capabilities {
                body: self.boolean()?,
                actions: self.boolean()?,
                activation: self.boolean()?,
                replacement: self.boolean()?,
                dismissal: self.boolean()?,
                permission_request: self.boolean()?,
                sound: self.boolean()?,
            }),
            1 => Response::Authorization(match self.tag()? {
                0 => Authorization::NotDetermined,
                1 => Authorization::Denied,
                2 => Authorization::Authorized,
                3 => Authorization::Provisional,
                4 => Authorization::NotRequired,
                _ => return Err(DecodeError::Malformed),
            }),
            2 => Response::Posted(self.notification_receipt()?),
            3 => Response::Replaced,
            4 => Response::DismissRequested,
            5 => Response::Events(self.list(MAX_EVENTS, |d| d.notification_event())?),
            6 => Response::Closed,
            7 => Response::Failed(self.notification_error()?),
            _ => return Err(DecodeError::Malformed),
        };
        if response.is_valid() {
            Ok(response)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}

pub fn decode_notification_request(bytes: &[u8]) -> Result<Request, DecodeError> {
    decode_drag_data(bytes, |d| d.notification_request())
}
pub fn decode_notification_response(bytes: &[u8]) -> Result<Response, DecodeError> {
    decode_drag_data(bytes, |d| d.notification_response())
}
pub fn decode_notification_event(bytes: &[u8]) -> Result<Event, DecodeError> {
    decode_drag_data(bytes, |d| d.notification_event())
}

//! Application-scoped notification service; callbacks publish owned wire values.
use crate::transport::Transport;
use gpuio_protocol::notification::{Error, Request, Response};
#[cfg(target_os = "macos")]
use gpuio_protocol::v1::Event;
use std::{cell::RefCell, rc::Rc, sync::Arc};

#[derive(Default)]
pub(crate) struct Host {
    closed: bool,
    #[cfg(target_os = "macos")]
    service: Option<crate::notification_macos::Service>,
}
pub(crate) type State = Rc<RefCell<Host>>;
impl Host {
    pub(crate) fn close(&mut self) {
        self.closed = true;
        #[cfg(target_os = "macos")]
        if let Some(mut service) = self.service.take() {
            service.close();
        }
    }
}
fn respond(transport: &Transport, correlation: i64, response: Response) {
    let accepted = transport
        .mailbox
        .lock()
        .expect("mailbox poisoned")
        .notification_response(correlation, response);
    if accepted {
        transport.wake_ocaml();
    }
}
pub(crate) fn dispatch(
    state: &State,
    correlation: i64,
    request: Request,
    transport: &Arc<Transport>,
) {
    let mut state = state.borrow_mut();
    if state.closed {
        respond(transport, correlation, Response::Failed(Error::Closed));
        return;
    }
    if matches!(request, Request::Close) {
        state.close();
        respond(transport, correlation, Response::Closed);
        return;
    }
    let identity = transport
        .desktop_inbox
        .lock()
        .expect("desktop inbox poisoned")
        .identity()
        .cloned();
    let Some(identity) = identity else {
        respond(transport, correlation, Response::Failed(Error::NotReady));
        return;
    };
    #[cfg(target_os = "macos")]
    {
        if state.service.is_none() {
            let weak = Arc::downgrade(transport);
            match crate::notification_macos::Service::new(&identity, move || {
                if let Some(transport) = weak.upgrade() {
                    transport
                        .mailbox
                        .lock()
                        .expect("mailbox poisoned")
                        .control(Event::NotificationPending);
                    transport.wake_ocaml();
                }
            }) {
                Ok(service) => state.service = Some(service),
                Err(error) => {
                    respond(transport, correlation, Response::Failed(error));
                    return;
                }
            }
        }
        let weak = Arc::downgrade(transport);
        state
            .service
            .as_mut()
            .unwrap()
            .request(correlation, request, move |response| {
                if let Some(transport) = weak.upgrade() {
                    respond(&transport, correlation, response);
                }
            });
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (identity, request);
        respond(transport, correlation, Response::Failed(Error::Unsupported));
    }
}

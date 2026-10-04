//! Main-thread desktop entry points; OS callbacks publish owned responses only.
use crate::{desktop_operations::Operations, desktop_state::DesktopState, transport::Transport};
use gpui::App;
use gpuio_protocol::{desktop as wire, v1::Event};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

pub(crate) struct HostState {
    inbox: Arc<Mutex<DesktopState>>,
    operations: Arc<Operations>,
    #[cfg(target_os = "linux")]
    services: crate::desktop_linux::Services,
}

#[must_use = "complete native desktop cleanup before stopping the application"]
pub(crate) struct Cleanup {
    #[cfg(target_os = "linux")]
    services: crate::desktop_linux::Cleanup,
}
impl Cleanup {
    pub(crate) async fn wait(self) {
        #[cfg(target_os = "linux")]
        self.services.wait().await;
    }
    pub(crate) fn wait_before_quit(self) {
        #[cfg(target_os = "linux")]
        self.services.wait_before_quit();
    }
}
impl HostState {
    pub(crate) fn close(&mut self) -> Cleanup {
        self.inbox.lock().expect("desktop inbox poisoned").close();
        self.operations.close();
        Cleanup {
            #[cfg(target_os = "linux")]
            services: self.services.close(),
        }
    }
}
pub(crate) type State = Rc<RefCell<HostState>>;

pub(crate) fn install(application: &gpui::Application, transport: Arc<Transport>) -> State {
    let state = Rc::new(RefCell::new(HostState {
        inbox: transport.desktop_inbox.clone(),
        operations: Arc::new(Operations::default()),
        #[cfg(target_os = "linux")]
        services: crate::desktop_linux::Services::default(),
    }));
    let incoming = state.clone();
    application.on_open_urls(move |links| {
        let mut notify = false;
        {
            let state = incoming.borrow();
            let mut inbox = state.inbox.lock().expect("desktop inbox poisoned");
            for link in links {
                notify |= inbox.push(link);
            }
        }
        if notify {
            transport
                .mailbox
                .lock()
                .expect("mailbox poisoned")
                .control(Event::DesktopPending);
            transport.wake_ocaml();
        }
    });
    state
}

fn capabilities(transport: &Transport) -> wire::Capabilities {
    wire::Capabilities {
        incoming_links: cfg!(target_os = "macos") || transport.desktop_instance_active(),
        runtime_registration: cfg!(target_os = "macos"),
        application_activation: cfg!(target_os = "macos"),
        file_reveal: true,
        file_open: true,
        document_metadata: cfg!(target_os = "macos"),
    }
}

pub(crate) fn dispatch(
    state: &State,
    correlation: i64,
    request: wire::Request,
    cx: &mut App,
    transport: &Arc<Transport>,
) {
    use wire::{Error, Request, Response};
    let respond = |response| transport.respond(Event::DesktopResponse(correlation, response));
    if !request.is_valid() {
        respond(Response::Failed(Error::InvalidRequest));
        return;
    }
    if matches!(request, Request::OpenFile(_) | Request::RegisterScheme(_))
        || (cfg!(target_os = "linux") && matches!(request, Request::RevealFile(_)))
    {
        let identity = match state
            .borrow()
            .inbox
            .lock()
            .expect("desktop inbox poisoned")
            .identity()
            .cloned()
        {
            Some(identity) => identity,
            None => {
                respond(Response::Failed(Error::NotReady));
                return;
            }
        };
        let weak = Arc::downgrade(transport);
        let admitted = state
            .borrow()
            .operations
            .admit(correlation, move |response| {
                if let Some(transport) = weak.upgrade() {
                    transport.respond_desktop(correlation, response);
                }
            });
        let ticket = match admitted {
            Ok(ticket) => ticket,
            Err(error) => {
                respond(Response::Failed(error));
                return;
            }
        };
        #[cfg(target_os = "macos")]
        match request {
            Request::OpenFile(path) => crate::desktop_macos::open(&path, ticket),
            Request::RegisterScheme(scheme) => {
                crate::desktop_macos::register(&identity, &scheme, ticket)
            }
            _ => unreachable!("asynchronous request partition"),
        }
        #[cfg(target_os = "linux")]
        {
            let _ = identity;
            match request {
                Request::OpenFile(path) => state.borrow().services.file(
                    ticket,
                    path,
                    gpuio_portal::desktop::FileOperation::Open,
                    correlation,
                ),
                Request::RevealFile(path) => state.borrow().services.file(
                    ticket,
                    path,
                    gpuio_portal::desktop::FileOperation::Reveal,
                    correlation,
                ),
                Request::RegisterScheme(_) => ticket.finish(Response::Failed(Error::Unsupported)),
                _ => unreachable!("asynchronous request partition"),
            }
        }
    } else {
        respond(request_immediate(state, request, cx, transport));
    }
}

fn request_immediate(
    state: &State,
    request: wire::Request,
    cx: &mut App,
    transport: &Transport,
) -> wire::Response {
    use wire::{Error, Request, Response};
    if !request.is_valid() {
        return Response::Failed(Error::InvalidRequest);
    }
    match request {
        Request::Capabilities => Response::Capabilities(capabilities(transport)),
        Request::ScrollbarPreference => scrollbar_preference(cfg!(target_os = "macos"), || {
            cx.should_auto_hide_scrollbars()
        }),
        Request::Configure(identity) => {
            let configured = state
                .borrow()
                .inbox
                .lock()
                .expect("desktop inbox poisoned")
                .configure(identity.clone());
            match configured {
                Ok(()) => {
                    cx.set_app_identity(&identity.identifier, &identity.name);
                    Response::Configured
                }
                Err(error) => Response::Failed(error),
            }
        }
        Request::TakeLinks => match state
            .borrow()
            .inbox
            .lock()
            .expect("desktop inbox poisoned")
            .take_links()
        {
            Ok(batch) => Response::Links(batch),
            Err(error) => Response::Failed(error),
        },
        Request::Activate(ignoring_other_apps) => {
            if state
                .borrow()
                .inbox
                .lock()
                .expect("desktop inbox poisoned")
                .identity()
                .is_none()
            {
                return Response::Failed(Error::NotReady);
            }
            if !capabilities(transport).application_activation {
                return Response::Failed(Error::Unsupported);
            }
            cx.activate(ignoring_other_apps);
            Response::Requested
        }
        Request::RevealFile(path) => {
            if state
                .borrow()
                .inbox
                .lock()
                .expect("desktop inbox poisoned")
                .identity()
                .is_none()
            {
                return Response::Failed(Error::NotReady);
            }
            #[cfg(target_os = "macos")]
            {
                crate::desktop_macos::reveal(&path)
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = path;
                Response::Failed(Error::Unsupported)
            }
        }
        Request::OpenFile(_) | Request::RegisterScheme(_) => {
            unreachable!("immediate request partition")
        }
    }
}

// The pinned Linux getter is a fixed default, not an OS preference. Do not read
// it or report it as AlwaysVisible. Keep native access lazy and main-thread only.
fn scrollbar_preference(supported: bool, read: impl FnOnce() -> bool) -> wire::Response {
    use wire::{Error, Response, ScrollbarPreference};
    if !supported {
        return Response::Failed(Error::Unsupported);
    }
    Response::ScrollbarPreference(if read() {
        ScrollbarPreference::AutoHide
    } else {
        ScrollbarPreference::AlwaysVisible
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "native-image-tests")]
    #[test]
    fn scrollbar_dispatch_preserves_correlation_without_identity_or_windows() {
        use gpuio_protocol::v1::Message;
        use std::os::{fd::AsRawFd, unix::net::UnixStream};

        let (_reader, writer) = UnixStream::pair().unwrap();
        let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
        let state = Rc::new(RefCell::new(HostState {
            inbox: transport.desktop_inbox.clone(),
            operations: Arc::new(Operations::default()),
            #[cfg(target_os = "linux")]
            services: crate::desktop_linux::Services::default(),
        }));
        let app = gpui::TestAppContext::single();
        for id in [7, 8] {
            let request = wire::Request::ScrollbarPreference;
            {
                let mut mailbox = transport.mailbox.lock().unwrap();
                mailbox
                    .submit(Message::Desktop(id, request.clone()), 3)
                    .unwrap();
                assert_eq!(mailbox.pop(), Some(Message::Desktop(id, request.clone())));
            }
            app.update(|cx| dispatch(&state, id, request, cx, &transport));
        }
        // TestPlatform deliberately reports false; this is dispatch evidence,
        // not a query of the developer's macOS setting.
        let response = if cfg!(target_os = "macos") {
            wire::Response::ScrollbarPreference(wire::ScrollbarPreference::AlwaysVisible)
        } else {
            wire::Response::Failed(wire::Error::Unsupported)
        };
        assert_eq!(
            transport.mailbox.lock().unwrap().drain(16),
            vec![
                Event::DesktopResponse(7, response.clone()),
                Event::DesktopResponse(8, response),
            ]
        );
        assert!(state.borrow().inbox.lock().unwrap().identity().is_none());
        assert!(!transport.mailbox.lock().unwrap().has_output());
    }

    #[test]
    fn scrollbar_query_reads_each_snapshot_and_never_reads_unsupported_backends() {
        use wire::{Error, Response, ScrollbarPreference::*};
        assert_eq!(
            scrollbar_preference(false, || panic!("unsupported backend read")),
            Response::Failed(Error::Unsupported)
        );
        let mut reads = 0;
        for (auto_hide, expected) in [(true, AutoHide), (false, AlwaysVisible), (true, AutoHide)] {
            let response = scrollbar_preference(true, || {
                reads += 1;
                auto_hide
            });
            assert_eq!(response, Response::ScrollbarPreference(expected));
        }
        assert_eq!(reads, 3, "snapshots must not cache a stale preference");
    }
}

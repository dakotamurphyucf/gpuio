//! Main-thread desktop entry points; OS callbacks publish owned responses only.
use crate::{desktop_operations::Operations, desktop_state::DesktopState, transport::Transport};
use gpui::App;
use gpuio_protocol::{desktop as wire, v1::Event};
use std::{cell::RefCell, rc::Rc, sync::Arc};

pub(crate) struct HostState {
    inbox: DesktopState,
    operations: Arc<Operations>,
}
impl HostState {
    pub(crate) fn close(&mut self) {
        self.inbox.close();
        self.operations.close();
    }
}
pub(crate) type State = Rc<RefCell<HostState>>;

pub(crate) fn install(application: &gpui::Application, transport: Arc<Transport>) -> State {
    let state = Rc::new(RefCell::new(HostState {
        inbox: DesktopState::default(),
        operations: Arc::new(Operations::default()),
    }));
    let incoming = state.clone();
    application.on_open_urls(move |links| {
        let mut notify = false;
        {
            let mut state = incoming.borrow_mut();
            for link in links {
                notify |= state.inbox.push(link);
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

fn capabilities() -> wire::Capabilities {
    wire::Capabilities {
        incoming_links: cfg!(target_os = "macos"),
        runtime_registration: cfg!(target_os = "macos"),
        application_activation: cfg!(target_os = "macos"),
        file_reveal: cfg!(target_os = "macos"),
        file_open: cfg!(target_os = "macos"),
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
    if matches!(request, Request::OpenFile(_) | Request::RegisterScheme(_)) {
        let identity = match state.borrow().inbox.identity().cloned() {
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
        #[cfg(not(target_os = "macos"))]
        {
            let _ = identity;
            ticket.finish(Response::Failed(Error::Unsupported));
        }
    } else {
        respond(request_immediate(state, request, cx));
    }
}

fn request_immediate(state: &State, request: wire::Request, cx: &mut App) -> wire::Response {
    use wire::{Error, Request, Response};
    if !request.is_valid() {
        return Response::Failed(Error::InvalidRequest);
    }
    match request {
        Request::Capabilities => Response::Capabilities(capabilities()),
        Request::Configure(identity) => {
            let mut state = state.borrow_mut();
            match state.inbox.configure(identity) {
                Ok(()) => {
                    let identity = state.inbox.identity().expect("configured identity");
                    cx.set_app_identity(&identity.identifier, &identity.name);
                    Response::Configured
                }
                Err(error) => Response::Failed(error),
            }
        }
        Request::TakeLinks => match state.borrow_mut().inbox.take_links() {
            Ok(batch) => Response::Links(batch),
            Err(error) => Response::Failed(error),
        },
        Request::Activate(ignoring_other_apps) => {
            if state.borrow().inbox.identity().is_none() {
                return Response::Failed(Error::NotReady);
            }
            if !capabilities().application_activation {
                return Response::Failed(Error::Unsupported);
            }
            cx.activate(ignoring_other_apps);
            Response::Requested
        }
        Request::RevealFile(path) => {
            if state.borrow().inbox.identity().is_none() {
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

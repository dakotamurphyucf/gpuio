//! Main-thread application identity and incoming-link integration. Other desktop
//! services are deliberately reported unsupported until their adapters exist.
use crate::{desktop_state::DesktopState, transport::Transport};
use gpui::App;
use gpuio_protocol::{desktop as wire, v1::Event};
use std::{cell::RefCell, rc::Rc, sync::Arc};

pub(crate) type State = Rc<RefCell<DesktopState>>;

pub(crate) fn install(application: &gpui::Application, transport: Arc<Transport>) -> State {
    let state = Rc::new(RefCell::new(DesktopState::default()));
    let incoming = state.clone();
    application.on_open_urls(move |links| {
        let mut notify = false;
        {
            let mut state = incoming.borrow_mut();
            for link in links {
                notify |= state.push(link);
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
        runtime_registration: false,
        application_activation: cfg!(target_os = "macos"),
        file_reveal: false,
        file_open: false,
        document_metadata: false,
    }
}

pub(crate) fn request(state: &State, request: wire::Request, cx: &mut App) -> wire::Response {
    use wire::{Error, Request, Response};
    if !request.is_valid() {
        return Response::Failed(Error::InvalidRequest);
    }
    match request {
        Request::Capabilities => Response::Capabilities(capabilities()),
        Request::Configure(identity) => {
            let mut state = state.borrow_mut();
            match state.configure(identity) {
                Ok(()) => {
                    let identity = state.identity().expect("configured identity");
                    cx.set_app_identity(&identity.identifier, &identity.name);
                    Response::Configured
                }
                Err(error) => Response::Failed(error),
            }
        }
        Request::TakeLinks => match state.borrow_mut().take_links() {
            Ok(batch) => Response::Links(batch),
            Err(error) => Response::Failed(error),
        },
        Request::Activate(ignoring_other_apps) => {
            if state.borrow().identity().is_none() {
                return Response::Failed(Error::NotReady);
            }
            if !capabilities().application_activation {
                return Response::Failed(Error::Unsupported);
            }
            cx.activate(ignoring_other_apps);
            Response::Requested
        }
        Request::RevealFile(_) | Request::OpenFile(_) | Request::RegisterScheme(_) => {
            Response::Failed(Error::Unsupported)
        }
    }
}

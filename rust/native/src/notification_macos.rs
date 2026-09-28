//! Bounded, explicit UNUserNotificationCenter ownership. No OCaml callbacks run
//! on delegate threads. The application must not also install GPUI's delegate.
use crate::notification_operations::{Operations, Ticket};
use crate::notification_state::{Signal, State, Token};
use block2::RcBlock;
use gpuio_protocol::{desktop::Identity, notification::*};
use objc2::{
    AnyThread, DefinedClass, MainThreadMarker, define_class, msg_send,
    rc::Retained,
    runtime::{Bool, ProtocolObject},
};
use objc2_foundation::{
    NSArray, NSBundle, NSError, NSObject, NSObjectProtocol, NSSet, NSString, NSUUID,
};
use objc2_user_notifications::*;
use std::sync::{Arc, Mutex};

struct Shared {
    state: Mutex<State>,
    operations: Arc<Operations>,
    nonce: String,
    notify: Box<dyn Fn() + Send + Sync>,
}
impl Shared {
    fn identifier(&self, receipt: &Receipt) -> String {
        format!("gpuio/{}/{}", self.nonce, receipt.id)
    }
    fn resolve(&self, identifier: &str) -> Option<Receipt> {
        let suffix = identifier
            .strip_prefix("gpuio/")?
            .strip_prefix(&self.nonce)?
            .strip_prefix('/')?;
        let id: i64 = suffix.parse().ok()?;
        if id <= 0 || id.to_string() != suffix {
            return None;
        }
        self.state
            .lock()
            .expect("notification state poisoned")
            .receipt(id)
    }
    fn finish(
        &self,
        token: &Token,
        result: Result<(), Error>,
        response: Response,
        ticket: &Ticket,
    ) {
        let (completion, retired) = {
            let mut state = self.state.lock().expect("notification state poisoned");
            let completion = state.complete(token, result);
            let retired = state.receipt(token.receipt.id).is_none();
            (completion, retired)
        };
        if completion.cleanup || (result.is_ok() && retired) {
            remove(&self.identifier(&token.receipt));
        }
        ticket.finish(match completion.result {
            Ok(()) => response,
            Err(error) => Response::Failed(error),
        });
        if completion.notify {
            (self.notify)();
        }
    }
    fn signal(&self, identifier: &str, signal: Signal) {
        let Some(receipt) = self.resolve(identifier) else {
            return;
        };
        let (notify, retired) = {
            let mut state = self.state.lock().expect("notification state poisoned");
            let notify = state.signal(&receipt, signal);
            (notify, state.receipt(receipt.id).is_none())
        };
        if retired {
            remove(identifier);
        }
        if notify {
            (self.notify)();
        }
    }
}

fn remove(identifier: &str) {
    let identifiers = NSArray::from_retained_slice(&[NSString::from_str(identifier)]);
    let center = UNUserNotificationCenter::currentNotificationCenter();
    center.removePendingNotificationRequestsWithIdentifiers(&identifiers);
    center.removeDeliveredNotificationsWithIdentifiers(&identifiers);
}
fn native_error(error: &NSError) -> Error {
    if error.domain().to_string() == "UNErrorDomain"
        && error.code() == UNErrorCode::NotificationsNotAllowed.0
    {
        Error::Denied
    } else {
        Error::NativeFailure
    }
}
fn authorization(settings: &UNNotificationSettings) -> Result<Authorization, Error> {
    match settings.authorizationStatus() {
        UNAuthorizationStatus::NotDetermined => Ok(Authorization::NotDetermined),
        UNAuthorizationStatus::Denied => Ok(Authorization::Denied),
        UNAuthorizationStatus::Authorized => Ok(Authorization::Authorized),
        UNAuthorizationStatus::Provisional => Ok(Authorization::Provisional),
        _ => Err(Error::Unsupported),
    }
}
fn probe(ticket: Ticket) {
    let callback = RcBlock::new(move |settings: std::ptr::NonNull<UNNotificationSettings>| {
        let result = authorization(unsafe { settings.as_ref() });
        ticket.finish(match result {
            Ok(value) => Response::Authorization(value),
            Err(error) => Response::Failed(error),
        });
    });
    UNUserNotificationCenter::currentNotificationCenter()
        .getNotificationSettingsWithCompletionHandler(&callback);
}
fn category_id(token: &Token, nonce: &str) -> String {
    format!("gpuio/{nonce}/{}/{}", token.receipt.id, token.serial)
}
fn action_key(token: &Token, id: &str) -> String {
    format!("{}/{id}", token.serial)
}
fn parse_action(key: &str) -> Option<Signal> {
    let (revision, id) = key.split_once('/')?;
    let parsed: i64 = revision.parse().ok()?;
    if parsed <= 0 || parsed.to_string() != revision || !valid_action_id(id) {
        return None;
    }
    Some(Signal::Action {
        revision: parsed,
        id: id.into(),
    })
}

pub(crate) struct Service {
    shared: Arc<Shared>,
    center: Retained<UNUserNotificationCenter>,
    delegate: Retained<ResponseDelegate>,
    closed: bool,
}
impl Service {
    pub(crate) fn new(
        identity: &Identity,
        notify: impl Fn() + Send + Sync + 'static,
    ) -> Result<Self, Error> {
        if MainThreadMarker::new().is_none() {
            return Err(Error::NativeFailure);
        }
        if !identity.is_valid() {
            return Err(Error::InvalidRequest);
        }
        if NSBundle::mainBundle()
            .bundleIdentifier()
            .is_none_or(|id| id.to_string() != identity.identifier)
        {
            return Err(Error::Unavailable);
        }
        let center = UNUserNotificationCenter::currentNotificationCenter();
        if center.delegate().is_some() {
            return Err(Error::Busy);
        }
        let shared = Arc::new(Shared {
            state: Mutex::new(State::default()),
            operations: Arc::new(Operations::default()),
            nonce: NSUUID::UUID().UUIDString().to_string(),
            notify: Box::new(notify),
        });
        let delegate = ResponseDelegate::new(shared.clone());
        center.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        Ok(Self {
            shared,
            center,
            delegate,
            closed: false,
        })
    }
    fn categories(&self) {
        // Main-thread calls only. Delegate callbacks can retire entries but never
        // register an older snapshot after a newer request. At most 144 entries;
        // the next mutation prunes retired categories and close removes them all.
        let contents = self
            .shared
            .state
            .lock()
            .expect("notification state poisoned")
            .contents();
        let categories: Vec<_> = contents
            .into_iter()
            .map(|(token, content)| {
                let actions: Vec<_> = content
                    .actions
                    .iter()
                    .map(|action| {
                        UNNotificationAction::actionWithIdentifier_title_options(
                            &NSString::from_str(&action_key(&token, &action.id)),
                            &NSString::from_str(&action.label),
                            UNNotificationActionOptions::empty(),
                        )
                    })
                    .collect();
                UNNotificationCategory::categoryWithIdentifier_actions_intentIdentifiers_options(
                    &NSString::from_str(&category_id(&token, &self.shared.nonce)),
                    &NSArray::from_retained_slice(&actions),
                    &NSArray::new(),
                    UNNotificationCategoryOptions::CustomDismissAction,
                )
            })
            .collect();
        self.center
            .setNotificationCategories(&NSSet::from_retained_slice(&categories));
    }
    pub(crate) fn request(
        &mut self,
        id: i64,
        request: Request,
        complete: impl FnOnce(Response) + Send + 'static,
    ) {
        if self.closed {
            complete(Response::Failed(Error::Closed));
            return;
        }
        if !request.is_valid() {
            complete(Response::Failed(Error::InvalidRequest));
            return;
        }
        if matches!(request, Request::Close) {
            self.close();
            complete(Response::Closed);
            return;
        }
        // Admission errors must also invoke the caller without retaining it.
        let callback = Arc::new(Mutex::new(Some(complete)));
        let deliver = callback.clone();
        let ticket = match self.shared.operations.admit(id, move |response| {
            if let Some(complete) = deliver
                .lock()
                .expect("notification callback poisoned")
                .take()
            {
                complete(response);
            }
        }) {
            Ok(ticket) => ticket,
            Err(error) => {
                if let Some(complete) = callback
                    .lock()
                    .expect("notification callback poisoned")
                    .take()
                {
                    complete(Response::Failed(error));
                }
                return;
            }
        };
        match request {
            Request::Capabilities => ticket.finish(Response::Capabilities(Capabilities {
                body: true,
                actions: true,
                activation: true,
                replacement: true,
                dismissal: true,
                permission_request: true,
                sound: true,
            })),
            Request::Authorization => probe(ticket),
            Request::RequestAuthorization => {
                let callback = RcBlock::new(move |_granted: Bool, error: *mut NSError| {
                    if let Some(error) = unsafe { error.as_ref() } {
                        ticket.finish(Response::Failed(native_error(error)));
                    } else {
                        probe(ticket.clone());
                    }
                });
                self.center
                    .requestAuthorizationWithOptions_completionHandler(
                        UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound,
                        &callback,
                    );
            }
            Request::TakeEvents => {
                let response = match self
                    .shared
                    .state
                    .lock()
                    .expect("notification state poisoned")
                    .take()
                {
                    Ok(events) => Response::Events(events),
                    Err(error) => Response::Failed(error),
                };
                ticket.finish(response);
            }
            Request::Post(tag, content) => {
                let token = self
                    .shared
                    .state
                    .lock()
                    .expect("notification state poisoned")
                    .post(tag, content.clone());
                match token {
                    Ok(token) => {
                        let response = Response::Posted(token.receipt.clone());
                        self.show(token, content, response, ticket);
                    }
                    Err(error) => ticket.finish(Response::Failed(error)),
                }
            }
            Request::Replace(receipt, content) => {
                let token = self
                    .shared
                    .state
                    .lock()
                    .expect("notification state poisoned")
                    .replace(&receipt, content.clone());
                match token {
                    Ok(token) => self.show(token, content, Response::Replaced, ticket),
                    Err(error) => ticket.finish(Response::Failed(error)),
                }
            }
            Request::Dismiss(receipt) => {
                let token = self
                    .shared
                    .state
                    .lock()
                    .expect("notification state poisoned")
                    .dismiss(&receipt);
                match token {
                    Ok(token) => {
                        remove(&self.shared.identifier(&receipt));
                        self.shared
                            .finish(&token, Ok(()), Response::DismissRequested, &ticket);
                        self.categories();
                    }
                    Err(error) => ticket.finish(Response::Failed(error)),
                }
            }
            Request::Close => unreachable!("handled before admission"),
        }
    }
    fn show(&self, token: Token, content: Content, response: Response, ticket: Ticket) {
        self.categories();
        let shared = self.shared.clone();
        let callback = RcBlock::new(move |settings: std::ptr::NonNull<UNNotificationSettings>| {
            let permission = authorization(unsafe { settings.as_ref() });
            let allowed = match permission {
                Ok(Authorization::Authorized | Authorization::Provisional) => Ok(()),
                Ok(Authorization::NotDetermined) => Err(Error::NotReady),
                Ok(Authorization::Denied) => Err(Error::Denied),
                Ok(Authorization::NotRequired) => Err(Error::Unsupported),
                Err(error) => Err(error),
            };
            if let Err(error) = allowed {
                shared.finish(&token, Err(error), response.clone(), &ticket);
                return;
            }
            // Shutdown may have retired this token while authorization was read.
            if shared
                .state
                .lock()
                .expect("notification state poisoned")
                .receipt(token.receipt.id)
                .is_none()
            {
                shared.finish(&token, Err(Error::Closed), response.clone(), &ticket);
                return;
            }
            let native = UNMutableNotificationContent::new();
            native.setTitle(&NSString::from_str(&content.title));
            native.setBody(&NSString::from_str(&content.body));
            native.setCategoryIdentifier(&NSString::from_str(&category_id(&token, &shared.nonce)));
            if content.sound == Sound::Default {
                native.setSound(Some(&UNNotificationSound::defaultSound()));
            }
            let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
                &NSString::from_str(&shared.identifier(&token.receipt)),
                &native,
                None,
            );
            let (shared, token, response, ticket) = (
                shared.clone(),
                token.clone(),
                response.clone(),
                ticket.clone(),
            );
            let completion = RcBlock::new(move |error: *mut NSError| {
                let result = match unsafe { error.as_ref() } {
                    Some(error) => Err(native_error(error)),
                    None => Ok(()),
                };
                shared.finish(&token, result, response.clone(), &ticket);
            });
            UNUserNotificationCenter::currentNotificationCenter()
                .addNotificationRequest_withCompletionHandler(&request, Some(&completion));
        });
        self.center
            .getNotificationSettingsWithCompletionHandler(&callback);
    }
    pub(crate) fn close(&mut self) {
        if self.closed {
            return;
        }
        self.closed = true;
        let receipts = {
            let mut state = self
                .shared
                .state
                .lock()
                .expect("notification state poisoned");
            let receipts = state.receipts();
            state.close();
            receipts
        };
        self.shared.operations.close();
        for receipt in receipts {
            remove(&self.shared.identifier(&receipt));
        }
        // Do not remove a delegate installed by a foreign component after us.
        if self.center.delegate().is_some_and(|delegate| {
            std::ptr::eq(
                &*delegate as *const _ as *const (),
                &*self.delegate as *const _ as *const (),
            )
        }) {
            self.center.setNotificationCategories(&NSSet::new());
            self.center.setDelegate(None);
        }
    }
}
impl Drop for Service {
    fn drop(&mut self) {
        self.close();
    }
}

struct DelegateIvars {
    shared: Arc<Shared>,
}
define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = DelegateIvars]
    struct ResponseDelegate;
    unsafe impl NSObjectProtocol for ResponseDelegate {}
    unsafe impl UNUserNotificationCenterDelegate for ResponseDelegate {
        #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
        fn response(
            &self,
            _center: &UNUserNotificationCenter,
            response: &UNNotificationResponse,
            completion: &block2::DynBlock<dyn Fn()>,
        ) {
            let identifier = response.notification().request().identifier().to_string();
            let action = response.actionIdentifier();
            let signal = if &*action == unsafe { UNNotificationDefaultActionIdentifier } {
                Some(Signal::Activated)
            } else if &*action == unsafe { UNNotificationDismissActionIdentifier } {
                Some(Signal::Closed(ClosedReason::User))
            } else {
                parse_action(&action.to_string())
            };
            if let Some(signal) = signal {
                self.ivars().shared.signal(&identifier, signal);
            }
            completion.call(());
        }
        #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
        fn presentation(
            &self,
            _center: &UNUserNotificationCenter,
            notification: &UNNotification,
            completion: &block2::DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
        ) {
            let request = notification.request();
            let known = self
                .ivars()
                .shared
                .resolve(&request.identifier().to_string())
                .is_some();
            let mut options = UNNotificationPresentationOptions::empty();
            if known {
                options = UNNotificationPresentationOptions::Banner
                    | UNNotificationPresentationOptions::List;
                if request.content().sound().is_some() {
                    options |= UNNotificationPresentationOptions::Sound;
                }
            }
            completion.call((options,));
        }
    }
);
impl ResponseDelegate {
    fn new(shared: Arc<Shared>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(DelegateIvars { shared });
        unsafe { msg_send![super(this), init] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn action_transport_keys_are_revisioned_and_strictly_parsed() {
        let token = Token {
            receipt: Receipt {
                id: 7,
                tag: "build".into(),
            },
            serial: 13,
        };
        assert_eq!(
            parse_action(&action_key(&token, "open")),
            Some(Signal::Action {
                revision: 13,
                id: "open".into()
            })
        );
        for key in [
            "0/open",
            "-1/open",
            "01/open",
            "1/",
            "1/a/b",
            "open",
            "1/日本語",
        ] {
            assert!(parse_action(key).is_none());
        }
    }
    #[test]
    fn unavailable_bundle_does_not_access_the_notification_center() {
        let identity = Identity {
            identifier: "com.example.gpuio-tests".into(),
            name: "Test".into(),
            schemes: vec![],
        };
        let result = Service::new(&identity, || ());
        assert!(matches!(
            result,
            Err(Error::NativeFailure | Error::Unavailable)
        ));
        // Exercise the request/close paths through the type checker without
        // launching or prompting in a unit-test process.
        if let Ok(mut service) = result {
            service.request(1, Request::Capabilities, |_| ());
            service.close();
        }
    }
}

//! AppKit requests retain only owned completion tickets across OS callbacks.
use crate::desktop_operations::Ticket;
use block2::RcBlock;
use gpuio_protocol::{
    desktop::{Error, Identity, Response},
    file_path::FilePath,
};
use objc2::{MainThreadMarker, rc::Retained, runtime::AnyObject};
use objc2_app_kit::{NSRunningApplication, NSWorkspace, NSWorkspaceOpenConfiguration};
use objc2_foundation::{NSArray, NSBundle, NSDictionary, NSError, NSString, NSURL};
use std::{ffi::OsStr, os::unix::ffi::OsStrExt, path::Path};

fn url(path: &FilePath) -> Result<Retained<NSURL>, Error> {
    NSURL::from_file_path(Path::new(OsStr::from_bytes(path.as_bytes())))
        .ok_or(Error::InvalidRequest)
}

fn classify(domain: &str, code: isize) -> Error {
    match (domain, code) {
        ("NSCocoaErrorDomain", 257 | 513 | 3072)
        | ("NSPOSIXErrorDomain", 1 | 13)
        | ("NSOSStatusErrorDomain", -54 | -10826) => Error::Denied,
        ("NSCocoaErrorDomain", 4 | 260)
        | ("NSPOSIXErrorDomain", 2)
        | ("NSOSStatusErrorDomain", -43 | -10814 | -10827) => Error::Unavailable,
        _ => Error::NativeFailure,
    }
}

fn error_response(error: &NSError) -> Response {
    fn inspect(error: &NSError, remaining: &mut usize) -> Error {
        if *remaining == 0 {
            return Error::NativeFailure;
        }
        *remaining -= 1;
        let classified = classify(&error.domain().to_string(), error.code());
        if classified != Error::NativeFailure {
            return classified;
        }
        // NSWorkspace can wrap a useful Launch Services error in a generic
        // Cocoa read error. Bound traversal even for cyclic/branching NSError data.
        for underlying in error.underlyingErrors().iter().take(*remaining) {
            let classified = inspect(&underlying, remaining);
            if classified != Error::NativeFailure {
                return classified;
            }
        }
        Error::NativeFailure
    }
    Response::Failed(inspect(error, &mut 8))
}

pub(crate) fn reveal(path: &FilePath) -> Response {
    if MainThreadMarker::new().is_none() {
        return Response::Failed(Error::NativeFailure);
    }
    let url = match url(path) {
        Ok(url) => url,
        Err(error) => return Response::Failed(error),
    };
    NSWorkspace::sharedWorkspace()
        .activateFileViewerSelectingURLs(&NSArray::from_slice(&[url.as_ref()]));
    // This API has no completion result. Requested never claims Finder selected
    // an item, nor that a missing/inaccessible path became available.
    Response::Requested
}

pub(crate) fn open(path: &FilePath, ticket: Ticket) {
    if MainThreadMarker::new().is_none() {
        ticket.finish(Response::Failed(Error::NativeFailure));
        return;
    }
    let url = match url(path) {
        Ok(url) => url,
        Err(error) => {
            ticket.finish(Response::Failed(error));
            return;
        }
    };
    let config = NSWorkspaceOpenConfiguration::configuration();
    config.setAddsToRecentItems(false);
    config.setPromptsUserIfNeeded(false);
    let callback = RcBlock::new(
        move |application: *mut NSRunningApplication, error: *mut NSError| {
            // AppKit lends these arguments only for the completion call. Convert the
            // NSError to an owned wire value before publishing across threads.
            let response = match unsafe { error.as_ref() } {
                Some(error) => error_response(error),
                None if !application.is_null() => Response::Requested,
                None => Response::Failed(Error::NativeFailure),
            };
            ticket.finish(response);
        },
    );
    NSWorkspace::sharedWorkspace().openURL_configuration_completionHandler(
        &url,
        &config,
        Some(&callback),
    );
}

fn bundle_declares(bundle: &NSBundle, scheme: &str) -> bool {
    let Some(types) = bundle.objectForInfoDictionaryKey(&NSString::from_str("CFBundleURLTypes"))
    else {
        return false;
    };
    let Some(types) = types.downcast_ref::<NSArray<AnyObject>>() else {
        return false;
    };
    let key = NSString::from_str("CFBundleURLSchemes");
    types.iter().take(64).any(|entry| {
        let Some(entry) = entry.downcast_ref::<NSDictionary<AnyObject, AnyObject>>() else {
            return false;
        };
        let Some(schemes) = entry.objectForKey(&key) else {
            return false;
        };
        let Some(schemes) = schemes.downcast_ref::<NSArray<AnyObject>>() else {
            return false;
        };
        schemes.iter().take(64).any(|value| {
            value
                .downcast_ref::<NSString>()
                .is_some_and(|value| value.to_string().eq_ignore_ascii_case(scheme))
        })
    })
}

pub(crate) fn register(identity: &Identity, scheme: &str, ticket: Ticket) {
    if MainThreadMarker::new().is_none() {
        ticket.finish(Response::Failed(Error::NativeFailure));
        return;
    }
    if !identity.schemes.iter().any(|declared| declared == scheme) {
        ticket.finish(Response::Failed(Error::InvalidRequest));
        return;
    }
    let bundle = NSBundle::mainBundle();
    if bundle
        .bundleIdentifier()
        .is_none_or(|id| id.to_string() != identity.identifier)
    {
        ticket.finish(Response::Failed(Error::Unavailable));
        return;
    }
    if !bundle_declares(&bundle, scheme) {
        ticket.finish(Response::Failed(Error::InvalidRequest));
        return;
    }
    let callback = RcBlock::new(move |error: *mut NSError| {
        let response = match unsafe { error.as_ref() } {
            Some(error) => error_response(error),
            None => Response::Registered,
        };
        ticket.finish(response);
    });
    NSWorkspace::sharedWorkspace()
        .setDefaultApplicationAtURL_toOpenURLsWithScheme_completionHandler(
            &bundle.bundleURL(),
            &NSString::from_str(scheme),
            Some(&callback),
        );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_errors_distinguish_denial_availability_and_unclassified_failures() {
        assert_eq!(classify("NSCocoaErrorDomain", 3072), Error::Denied);
        assert_eq!(classify("NSCocoaErrorDomain", 260), Error::Unavailable);
        assert_eq!(classify("NSOSStatusErrorDomain", -10826), Error::Denied);
        assert_eq!(
            classify("NSOSStatusErrorDomain", -10814),
            Error::Unavailable
        );
        assert_eq!(classify("custom", 260), Error::NativeFailure);
    }
}

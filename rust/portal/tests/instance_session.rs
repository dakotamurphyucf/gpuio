//! Run explicitly under dbus-run-session; never depend on a developer's bus.
use futures_lite::future::block_on;
use gpuio_portal::instance::{self, Launch};
use gpuio_protocol::desktop::Error;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

struct CancelOnDrop(async_channel::Sender<()>);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.close();
    }
}

#[test]
#[ignore = "requires an explicitly isolated dbus-run-session"]
fn concurrent_connections_have_one_owner_and_forward_without_duplicates() {
    assert_eq!(std::env::var("GPUIO_PRIVATE_BUS_TEST").as_deref(), Ok("1"));
    let identifier = format!("com.gpuio.instance-test.p{}", std::process::id());
    let accepted = Arc::new(Mutex::new(Vec::new()));
    let full = Arc::new(AtomicBool::new(false));
    let admit: instance::Admit = {
        let accepted = accepted.clone();
        let full = full.clone();
        Arc::new(move |links| {
            if full.load(Ordering::Acquire) {
                return Err(Error::Busy);
            }
            accepted.lock().unwrap().push(links);
            Ok(())
        })
    };
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::scope(|scope| {
        // This guard is inside the scope, so an assertion cancels servers before
        // scoped threads are joined during unwinding.
        let (cancel, cancelled) = async_channel::bounded(1);
        let cancel = CancelOnDrop(cancel);
        for index in 0..8 {
            let identifier = &identifier;
            let admit = admit.clone();
            let cancelled = cancelled.clone();
            let send = send.clone();
            scope.spawn(move || {
                match block_on(instance::launch(
                    identifier,
                    vec![format!("launch-{index}")],
                    admit,
                )) {
                    Ok(Launch::Primary(server)) => {
                        send.send((index, Ok(true))).unwrap();
                        assert_eq!(block_on(server.serve(cancelled)), Ok(()));
                    }
                    Ok(Launch::Forwarded) => {
                        send.send((index, Ok(false))).unwrap();
                    }
                    Err(error) => {
                        send.send((index, Err(error))).unwrap();
                    }
                }
            });
        }
        let mut primary = None;
        for _ in 0..8 {
            let (index, result) = receive.recv_timeout(Duration::from_secs(15)).unwrap();
            if result.unwrap() {
                assert!(primary.replace(index).is_none());
            }
        }
        let primary = primary.expect("one bus owner");
        {
            let accepted = accepted.lock().unwrap();
            assert_eq!(accepted.len(), 8);
            assert_eq!(accepted[0], [format!("launch-{primary}")]);
            let all: std::collections::BTreeSet<_> = accepted.iter().flatten().collect();
            assert_eq!(all.len(), 8);
        }
        assert!(matches!(
            block_on(instance::launch(&identifier, vec![], admit.clone())),
            Ok(Launch::Forwarded)
        ));
        assert!(accepted.lock().unwrap().last().unwrap().is_empty());
        full.store(true, Ordering::Release);
        assert!(matches!(
            block_on(instance::launch(
                &identifier,
                vec!["refused".into()],
                admit.clone()
            )),
            Err(Error::Busy)
        ));
        assert_eq!(accepted.lock().unwrap().len(), 9);
        drop(cancel);
    });
    // A closed server really releases its name; no queued secondary inherits it.
    full.store(false, Ordering::Release);
    let result = block_on(instance::launch(
        &identifier,
        vec!["replacement".into()],
        admit,
    ))
    .unwrap();
    let Launch::Primary(server) = result else {
        panic!("old owner retained its name")
    };
    let (cancel, receiver) = async_channel::bounded(1);
    cancel.close();
    assert_eq!(block_on(server.serve(receiver)), Ok(()));
    assert_eq!(accepted.lock().unwrap().last().unwrap(), &["replacement"]);
}

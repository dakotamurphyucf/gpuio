use binprot::BinProtWrite;
use gpuio_native::mailbox::Mailbox;
use gpuio_protocol::{WindowId, file_path::FilePath, v1::*};

#[test]
fn selected_paths_count_toward_response_envelope_and_window_lifetime() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let path = FilePath::new(vec![b'/'; 16_384]).unwrap();
    let config = FileDialogConfig::Open(OpenFileConfig {
        selection: FileSelection::Files,
        multiple: true,
        title: "Open".into(),
        accept_label: "Open".into(),
        directory: None,
    });
    let mut mailbox = Mailbox::default();
    for correlation in 1..=8 {
        mailbox
            .submit(Message::FileDialog(correlation, window, config.clone()), 64)
            .unwrap();
        assert!(mailbox.pop().is_some());
        mailbox.respond(Event::FileDialogResult(
            correlation,
            window,
            FileDialogResult::Selected(vec![path.clone(); 16]),
        ));
    }
    assert!(mailbox.has_window_output(window.slot()));
    let mut count = 0;
    while mailbox.has_output() {
        let events = mailbox.drain(256);
        assert!(!events.is_empty());
        assert!(events.len() <= 3);
        count += events.len();
        let mut encoded = vec![];
        events.binprot_write(&mut encoded).unwrap();
        assert!(encoded.len() <= MAX_MESSAGE_BYTES);
    }
    assert_eq!(count, 8);
    assert!(!mailbox.has_window_output(window.slot()));
}

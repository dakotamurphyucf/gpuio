use gpuio_protocol::{WindowId, file_path::FilePath, v1::*};

pub fn window() -> WindowId {
    WindowId::from_parts(2, 3).unwrap()
}
pub fn directory() -> FilePath {
    FilePath::new(b"/tmp/\xff".to_vec()).unwrap()
}
pub fn requests() -> Vec<Message> {
    vec![
        Message::FileDialog(
            128,
            window(),
            FileDialogConfig::Open(OpenFileConfig {
                selection: FileSelection::FilesAndDirectories,
                multiple: true,
                title: "Open files".into(),
                accept_label: "Choose".into(),
                directory: Some(directory()),
            }),
        ),
        Message::FileDialog(
            129,
            window(),
            FileDialogConfig::Save(SaveFileConfig {
                directory: directory(),
                suggested_name: "report.sql.s".into(),
                title: "Save report".into(),
                accept_label: "Save".into(),
            }),
        ),
    ]
}
pub fn events() -> Vec<Event> {
    let mut events = vec![
        Event::FileDialogResult(
            128,
            window(),
            FileDialogResult::Selected(vec![
                FilePath::new(b"/tmp/\xff/report.txt".to_vec()).unwrap(),
            ]),
        ),
        Event::FileDialogResult(129, window(), FileDialogResult::Cancelled),
    ];
    events.extend(
        [
            FileDialogError::InvalidRequest,
            FileDialogError::Unsupported,
            FileDialogError::Busy,
            FileDialogError::Closed,
            FileDialogError::NotReady,
            FileDialogError::NativeFailure,
            FileDialogError::LimitExceeded,
        ]
        .into_iter()
        .enumerate()
        .map(|(i, error)| {
            Event::FileDialogResult(130 + i as i64, window(), FileDialogResult::Failed(error))
        }),
    );
    events
}

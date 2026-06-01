use std::{fs, path::Path};

#[test]
fn rust_code_uses_request_and_source_vocabulary() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();
    scan_rust_files(&src, &mut |path, line_number, line| {
        if line.contains("broadcast::channel") {
            return;
        }

        for banned in [
            "EventStatus",
            "EventResult",
            "EventUserResult",
            "CreateEvent",
            "ListEvents",
            "ActionSubmission",
            "create_event",
            "get_event",
            "list_events",
            "event_visible",
            "expire_due_events",
            "submit_action",
            "channel_id",
            "get_channel",
            "list_channels",
            "create_channel",
            "delete_channel",
            "clear_channel",
        ] {
            if line.contains(banned) {
                violations.push(format!("{}:{line_number}: {banned}", path.display()));
            }
        }
    });

    assert!(
        violations.is_empty(),
        "old request/source vocabulary remains:\n{}",
        violations.join("\n")
    );
}

fn scan_rust_files(dir: &Path, visit: &mut impl FnMut(&Path, usize, &str)) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            scan_rust_files(&path, visit);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let contents = fs::read_to_string(&path).unwrap();
        for (index, line) in contents.lines().enumerate() {
            visit(&path, index + 1, line);
        }
    }
}

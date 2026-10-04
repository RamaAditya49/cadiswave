use cadiswave_runtime::events::{EventBuffer, EventTail};
use std::time::{Duration, Instant};
#[test]
fn event_storage_caps_lines_and_entries() {
    let mut buffer = EventBuffer::default();
    for i in 0..500 {
        buffer.push(format!("{i}:{}", "x".repeat(6000)));
    }
    let lines = buffer.latest();
    assert_eq!(lines.len(), 200);
    assert!(lines.iter().all(|line| line.len() <= 4096));
    assert!(lines[0].starts_with("300:"));
}
#[test]
fn real_journal_reader_has_repeated_bounded_shutdown() {
    for _ in 0..3 {
        let mut tail = EventTail::start().unwrap();
        let start = Instant::now();
        tail.stop().unwrap();
        assert!(start.elapsed() < Duration::from_secs(3));
        tail.stop().unwrap();
        assert!(tail.latest().len() <= 200);
    }
}

use forge::runtime::event::EventRecord;
use forge::runtime::session::{CheckpointRecord, SessionSnapshot};

fn fixture_value(path: &str) -> serde_json::Value {
    let data = std::fs::read_to_string(path).expect("read fixture");
    serde_json::from_str(&data).expect("parse fixture json")
}

#[test]
fn event_record_fixture_roundtrip_is_stable() {
    let fixture_path = format!(
        "{}/tests/golden/event_record_v1.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let fixture = fixture_value(&fixture_path);
    let record: EventRecord = serde_json::from_value(fixture.clone()).expect("decode event record");

    let encoded = serde_json::to_value(&record).expect("encode event record");
    assert_eq!(encoded, fixture);
}

#[test]
fn checkpoint_record_fixture_roundtrip_is_stable() {
    let fixture_path = format!(
        "{}/tests/golden/checkpoint_record_v1.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let fixture = fixture_value(&fixture_path);
    let record: CheckpointRecord =
        serde_json::from_value(fixture.clone()).expect("decode checkpoint record");

    assert_eq!(record.run_id, "run-1");
    assert_eq!(record.next_node, "review");
    assert_eq!(record.pending_interrupts.len(), 1);

    let encoded = serde_json::to_value(&record).expect("encode checkpoint record");
    assert_eq!(encoded, fixture);
}

#[test]
fn session_snapshot_fixture_roundtrip_is_stable() {
    let fixture_path = format!(
        "{}/tests/golden/session_snapshot_v1.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let fixture = fixture_value(&fixture_path);
    let snapshot: SessionSnapshot =
        serde_json::from_value(fixture.clone()).expect("decode session snapshot");

    assert_eq!(snapshot.version, 1);
    assert_eq!(snapshot.session_id, "session-1");
    assert_eq!(snapshot.messages.len(), 2);
    assert_eq!(snapshot.compactions.len(), 1);

    let encoded = serde_json::to_value(&snapshot).expect("encode session snapshot");
    assert_eq!(encoded, fixture);
}

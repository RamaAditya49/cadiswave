use cadiswave_core::pipewire_dump::parse;
use serde_json::json;
#[test]
fn snapshot_removal_then_reuse_retains_the_new_identity() {
    let output = br#"[{"id":0,"type":"PipeWire:Interface:Core","info":{"cookie":12}}, {"id":9,"type":"PipeWire:Interface:Node","info":{"props":{"object.serial":10}}}]
[{"id":9,"info":null}]
[{"id":9,"type":"PipeWire:Interface:Node","info":{"props":{"object.serial":11}}}]"#;
    let graph = parse(output).unwrap();
    assert_eq!(graph.as_array().unwrap().len(), 2);
    assert_eq!(graph[1]["info"]["props"]["object.serial"], 11);
}
#[test]
fn removal_before_snapshot_does_not_delete_a_later_object() {
    assert_eq!(
        parse(br#"[{"id":9}] [{"id":9,"type":"PipeWire:Interface:Node"}]"#).unwrap(),
        json!([{"id":9,"type":"PipeWire:Interface:Node"}])
    );
}
#[test]
fn malformed_trailing_data_never_publishes_a_partial_graph() {
    for bytes in [
        b"[] [".as_slice(),
        b"[] false",
        b"[] [{\"id\":9,\"info\":{}}]",
        b"[] [{\"type\":\"PipeWire:Interface:Node\"}]",
        b"",
    ] {
        assert!(parse(bytes).is_err(), "accepted malformed batch");
    }
}
#[test]
fn one_snapshot_retains_duplicate_records_for_consumer_validation() {
    let output = json!([{"id":0,"type":"PipeWire:Interface:Core"},{"id":0,"type":"PipeWire:Interface:Core"}]);
    assert_eq!(
        parse(&serde_json::to_vec(&output).unwrap()).unwrap(),
        output
    );
}

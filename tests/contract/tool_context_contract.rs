use std::path::PathBuf;

#[test]
fn contract_includes_permission_abort_and_attachment_endpoints() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("specs/001-tool-context/contracts/tool-context.openapi.yaml");
    let contents = std::fs::read_to_string(&path).expect("contract file");

    assert!(contents.contains("/runs/{run_id}/permissions"));
    assert!(contents.contains("/runs/{run_id}/abort"));
    assert!(contents.contains("/runs/{run_id}/attachments/{attachment_id}"));
}

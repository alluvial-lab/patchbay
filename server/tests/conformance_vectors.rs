use std::{collections::BTreeMap, env, fs, path::PathBuf};

use serde::Deserialize;
use serde_json::Value;

const RUNNER: &str = "rust-server";

#[derive(Debug, Deserialize)]
struct ConformanceVector {
    vector_id: String,
    property_id: String,
    promotion_status: String,
    #[serde(default)]
    implementation_checks: Vec<ImplementationCheck>,
    input: Value,
    expected_outcome: Value,
}

#[derive(Debug, Deserialize)]
struct ImplementationCheck {
    runner: String,
    case: String,
}

#[derive(Debug, Deserialize)]
struct RequestedCheck {
    vector_id: String,
    case: String,
}

fn vectors() -> BTreeMap<String, ConformanceVector> {
    let vector_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../contracts/vectors");
    let mut files = fs::read_dir(vector_dir)
        .expect("conformance vector directory must be readable")
        .map(|entry| entry.expect("vector directory entry must be readable").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "json"))
        .collect::<Vec<_>>();
    files.sort();

    files
        .into_iter()
        .map(|path| {
            let vector: ConformanceVector = serde_json::from_slice(
                &fs::read(&path).expect("conformance vector must be readable"),
            )
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            (vector.vector_id.clone(), vector)
        })
        .collect()
}

fn requests() -> Vec<RequestedCheck> {
    env::var("PATCHBAY_CONFORMANCE_REQUESTS")
        .ok()
        .map(|raw| serde_json::from_str(&raw).expect("requested checks must be valid JSON"))
        .unwrap_or_default()
}

async fn execute_case(vector: &ConformanceVector, case: &str) -> Result<(), String> {
    let _claimed_fields = (&vector.property_id, &vector.promotion_status, &vector.input, &vector.expected_outcome);
    Err(format!("unhandled {RUNNER} conformance case {}:{case}", vector.vector_id))
}

#[tokio::test]
async fn conformance_vector_runner() {
    let vectors = vectors();
    for request in requests() {
        let vector = vectors
            .get(&request.vector_id)
            .unwrap_or_else(|| panic!("unknown vector id {}", request.vector_id));
        assert!(
            vector.implementation_checks.iter().any(|check| {
                check.runner == RUNNER && check.case == request.case
            }),
            "unregistered requested check {}:{}",
            request.vector_id,
            request.case,
        );
        execute_case(vector, &request.case)
            .await
            .unwrap_or_else(|error| panic!("{error}"));
        println!(
            "PATCHBAY_CONFORMANCE_EXECUTED={}:{}",
            request.vector_id, request.case
        );
    }
}

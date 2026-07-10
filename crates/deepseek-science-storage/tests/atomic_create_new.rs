use deepseek_science_storage::{AtomicWriteRequest, StorageError, StorageRoot};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Barrier};
use std::thread;

#[test]
fn concurrent_create_new_writes_publish_exactly_one_complete_payload() {
    const PAYLOAD_A: &[u8] = b"writer-a\n";
    const PAYLOAD_B: &[u8] = b"writer-b\n";

    let directory = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "deepseek-science-storage-create-new-race-{}",
        std::process::id()
    ));
    fs::create_dir(&directory).expect("test directory should be unique");

    let target_path = directory.join("result.bin");
    assert!(!target_path.exists());

    let root = StorageRoot::new(directory.clone()).expect("test root should be valid");
    let plan_a = AtomicWriteRequest::new("result.bin", PAYLOAD_A)
        .plan(&root)
        .expect("first safe path should plan");
    let plan_b = AtomicWriteRequest::new("result.bin", PAYLOAD_B)
        .plan(&root)
        .expect("second safe path should plan");
    let temp_path = plan_a.temp_path().to_path_buf();
    let barrier = Arc::new(Barrier::new(2));

    let barrier_a = Arc::clone(&barrier);
    let writer_a = thread::spawn(move || {
        barrier_a.wait();
        (PAYLOAD_A, plan_a.execute(PAYLOAD_A))
    });
    let barrier_b = Arc::clone(&barrier);
    let writer_b = thread::spawn(move || {
        barrier_b.wait();
        (PAYLOAD_B, plan_b.execute(PAYLOAD_B))
    });

    let (payload_a, result_a) = writer_a.join().expect("first writer should join");
    let (payload_b, result_b) = writer_b.join().expect("second writer should join");
    assert_eq!(result_a.is_ok() as usize + result_b.is_ok() as usize, 1);
    assert_eq!(result_a.is_err() as usize + result_b.is_err() as usize, 1);

    let (successful_payload, failure) = match (&result_a, &result_b) {
        (Ok(()), Err(error)) => (payload_a, error),
        (Err(error), Ok(())) => (payload_b, error),
        _ => panic!("exactly one concurrent writer should succeed"),
    };
    assert!(matches!(
        failure,
        StorageError::TargetAlreadyExists { .. } | StorageError::WriteFailed { .. }
    ));

    let final_bytes = fs::read(&target_path).expect("final target should be readable");
    assert!(final_bytes == PAYLOAD_A || final_bytes == PAYLOAD_B);
    assert_eq!(final_bytes, successful_payload);
    assert!(!temp_path.exists());

    let remaining_paths = fs::read_dir(&directory)
        .expect("test directory should be readable")
        .map(|entry| entry.expect("test entry should be readable").path())
        .collect::<Vec<_>>();
    assert_eq!(remaining_paths, vec![target_path.clone()]);

    fs::remove_file(&target_path).expect("target cleanup should succeed");
    fs::remove_dir(&directory).expect("test directory cleanup should succeed");
}

use deepseek_science_storage::{AtomicWriteRequest, StorageError, StorageRoot, WriteMode};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;

static NEXT_TEST_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

fn create_test_directory(label: &str) -> PathBuf {
    let sequence = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
    let directory = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "deepseek-science-storage-{label}-{}-{sequence}",
        std::process::id()
    ));
    fs::create_dir(&directory).expect("test directory should be unique");

    directory
}

fn directory_paths(directory: &Path) -> Vec<PathBuf> {
    fs::read_dir(directory)
        .expect("test directory should be readable")
        .map(|entry| entry.expect("test entry should be readable").path())
        .collect()
}

#[test]
fn execute_creates_exact_opaque_and_empty_bytes_and_removes_temp_files() {
    let directory = create_test_directory("exact-bytes");
    let root = StorageRoot::new(directory.clone()).expect("test root should be valid");
    let request = AtomicWriteRequest::new("result.bin", vec![0, 159, 255, 10]);
    let plan = request.plan(&root).expect("safe path should plan");
    let empty_request = AtomicWriteRequest::new("empty.bin", Vec::<u8>::new());
    let empty_plan = empty_request.plan(&root).expect("safe path should plan");

    plan.execute(request.content())
        .expect("create-new write should succeed");
    empty_plan
        .execute(empty_request.content())
        .expect("empty create-new write should succeed");

    assert_eq!(
        fs::read(plan.target_path()).expect("target should be readable"),
        request.content()
    );
    assert_eq!(
        fs::read(empty_plan.target_path()).expect("empty target should be readable"),
        empty_request.content()
    );
    assert!(!plan.temp_path().exists());
    assert!(!empty_plan.temp_path().exists());
    let remaining_paths = directory_paths(&directory);
    assert_eq!(remaining_paths.len(), 2);
    assert!(remaining_paths.contains(&plan.target_path().to_path_buf()));
    assert!(remaining_paths.contains(&empty_plan.target_path().to_path_buf()));

    fs::remove_file(plan.target_path()).expect("target cleanup should succeed");
    fs::remove_file(empty_plan.target_path()).expect("empty target cleanup should succeed");
    fs::remove_dir(&directory).expect("test directory cleanup should succeed");
}

#[test]
fn execute_refuses_existing_target_without_modifying_it() {
    const SENTINEL: &[u8] = b"existing";

    let directory = create_test_directory("existing-target");
    let root = StorageRoot::new(directory.clone()).expect("test root should be valid");
    let request = AtomicWriteRequest::new("result.bin", b"new".to_vec());
    let plan = request.plan(&root).expect("safe path should plan");
    fs::write(plan.target_path(), SENTINEL).expect("target setup should succeed");

    let error = plan
        .execute(request.content())
        .expect_err("existing target should be refused");

    assert!(matches!(error, StorageError::TargetAlreadyExists { .. }));
    assert_eq!(
        fs::read(plan.target_path()).expect("existing target should remain readable"),
        SENTINEL
    );
    assert!(!plan.temp_path().exists());
    assert_eq!(
        directory_paths(&directory),
        vec![plan.target_path().to_path_buf()]
    );

    fs::remove_file(plan.target_path()).expect("target cleanup should succeed");
    fs::remove_dir(&directory).expect("test directory cleanup should succeed");
}

#[test]
fn execute_requires_existing_parent_without_creating_it() {
    let directory = create_test_directory("missing-parent");
    let root = StorageRoot::new(directory.clone()).expect("test root should be valid");
    let request = AtomicWriteRequest::new("missing/result.bin", b"new".to_vec());
    let plan = request.plan(&root).expect("safe path should plan");

    let error = plan
        .execute(request.content())
        .expect_err("missing parent should be refused");

    assert!(matches!(error, StorageError::ParentDirectoryMissing { .. }));
    assert!(!directory.join("missing").exists());
    assert!(directory_paths(&directory).is_empty());

    fs::remove_dir(&directory).expect("test directory cleanup should succeed");
}

#[test]
fn execute_does_not_replace_or_remove_a_stale_temp_file() {
    const STALE: &[u8] = b"stale";

    let directory = create_test_directory("stale-temp");
    let root = StorageRoot::new(directory.clone()).expect("test root should be valid");
    let request = AtomicWriteRequest::new("result.bin", b"new".to_vec());
    let plan = request.plan(&root).expect("safe path should plan");
    fs::write(plan.temp_path(), STALE).expect("temp setup should succeed");

    let error = plan
        .execute(request.content())
        .expect_err("stale temp should be refused");

    assert!(matches!(error, StorageError::WriteFailed { .. }));
    assert_eq!(
        fs::read(plan.temp_path()).expect("stale temp should remain readable"),
        STALE
    );
    assert!(!plan.target_path().exists());
    assert_eq!(
        directory_paths(&directory),
        vec![plan.temp_path().to_path_buf()]
    );

    fs::remove_file(plan.temp_path()).expect("temp cleanup should succeed");
    fs::remove_dir(&directory).expect("test directory cleanup should succeed");
}

#[test]
fn execute_rejects_replace_existing_without_filesystem_changes() {
    let directory = create_test_directory("replace-existing");
    let root = StorageRoot::new(directory.clone()).expect("test root should be valid");
    let request = AtomicWriteRequest::new("result.bin", b"new".to_vec())
        .with_write_mode(WriteMode::ReplaceExisting);
    let plan = request.plan(&root).expect("safe path should plan");

    let error = plan
        .execute(request.content())
        .expect_err("replace-existing execution should be deferred");

    assert!(matches!(error, StorageError::Backend { .. }));
    assert!(!plan.target_path().exists());
    assert!(!plan.temp_path().exists());
    assert!(directory_paths(&directory).is_empty());

    fs::remove_dir(&directory).expect("test directory cleanup should succeed");
}

#[test]
fn concurrent_create_new_writes_publish_exactly_one_complete_payload() {
    const PAYLOAD_A: &[u8] = b"writer-a\n";
    const PAYLOAD_B: &[u8] = b"writer-b\n";

    let directory = create_test_directory("create-new-race");

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

    assert_eq!(directory_paths(&directory), vec![target_path.clone()]);

    fs::remove_file(&target_path).expect("target cleanup should succeed");
    fs::remove_dir(&directory).expect("test directory cleanup should succeed");
}

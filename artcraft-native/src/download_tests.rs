use super::{unique_destination, unique_destination_except};

#[test]
fn duplicate_batch_names_are_reserved_before_any_download_finishes() {
  let dir = tempfile::tempdir().unwrap();
  std::fs::write(dir.path().join("artcraft_abc (1).png"), b"old").unwrap();
  let mut chosen = Vec::new();
  for expected in ["artcraft_abc.png", "artcraft_abc (2).png", "artcraft_abc (3).png"] {
    let path = unique_destination_except(dir.path(), "artcraft_abc.png", &chosen);
    assert_eq!(path, dir.path().join(expected));
    assert!(!path.exists(), "reservations must work before files are created");
    chosen.push(path);
  }
  assert_eq!(std::fs::read(dir.path().join("artcraft_abc (1).png")).unwrap(), b"old");
}

#[test]
fn duplicate_extensionless_batch_names_are_reserved() {
  let dir = tempfile::tempdir().unwrap();
  let first = unique_destination_except(dir.path(), "README", &[]);
  assert_eq!(unique_destination_except(dir.path(), "README", &[first]), dir.path().join("README (1)"));
}

#[test]
fn a_directory_counts_as_taken() {
  let dir = tempfile::tempdir().unwrap();
  std::fs::create_dir(dir.path().join("artcraft_abc.png")).unwrap();
  assert_eq!(unique_destination(dir.path(), "artcraft_abc.png"), dir.path().join("artcraft_abc (1).png"));
}

#[cfg(unix)]
#[test]
fn a_dangling_symlink_counts_as_taken() {
  let dir = tempfile::tempdir().unwrap();
  let missing = dir.path().join("missing.png");
  let link = dir.path().join("artcraft_abc.png");
  std::os::unix::fs::symlink(&missing, &link).unwrap();
  assert_eq!(unique_destination(dir.path(), "artcraft_abc.png"), dir.path().join("artcraft_abc (1).png"));
  assert_eq!(std::fs::read_link(link).unwrap(), missing);
}

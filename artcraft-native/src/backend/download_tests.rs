use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::mpsc::channel;
use std::sync::{Arc, RwLock};
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::{Duration, Instant};

use super::{ApiHost, Backend, Event, JobStore, ProviderAccounts};

const BODY: &[u8] = b"downloaded bytes";
const TIMEOUT: Duration = Duration::from_secs(10);

#[test]
fn a_batch_download_creates_a_new_file_and_reports_saved() {
  let dir = tempfile::tempdir().unwrap();
  let path = dir.path().join("artcraft_abc.png");
  let backend = backend(dir.path());
  let (url, server) = serve("200 OK", || {});
  backend.save_url_new(url, path.clone());
  assert!(matches!(backend.rx.recv_timeout(TIMEOUT).unwrap(), Event::Saved(saved) if saved == path));
  server.join().unwrap();
  assert_eq!(std::fs::read(path).unwrap(), BODY);
}

#[test]
fn a_batch_download_refuses_an_existing_file() {
  let dir = tempfile::tempdir().unwrap();
  let path = dir.path().join("artcraft_abc.png");
  std::fs::write(&path, b"sentinel").unwrap();
  let backend = backend(dir.path());
  let (url, server) = serve("200 OK", || {});
  backend.save_url_new(url, path.clone());
  assert_failed(&backend);
  server.join().unwrap();
  assert_eq!(std::fs::read(path).unwrap(), b"sentinel");
}

#[test]
fn a_file_appearing_during_the_download_is_not_overwritten() {
  let dir = tempfile::tempdir().unwrap();
  let path = dir.path().join("artcraft_abc.png");
  assert!(std::fs::symlink_metadata(&path).is_err());
  let backend = backend(dir.path());
  let race_path = path.clone();
  let (url, server) = serve("200 OK", move || std::fs::write(race_path, b"sentinel").unwrap());
  backend.save_url_new(url, path.clone());
  assert_failed(&backend);
  server.join().unwrap();
  assert_eq!(std::fs::read(path).unwrap(), b"sentinel");
}

#[cfg(unix)]
#[test]
fn a_symlink_appearing_during_the_download_does_not_touch_its_target() {
  let outside = tempfile::tempdir().unwrap();
  let sentinel = outside.path().join("sentinel.png");
  std::fs::write(&sentinel, b"sentinel").unwrap();
  let dir = tempfile::tempdir().unwrap();
  let path = dir.path().join("artcraft_abc.png");
  let backend = backend(dir.path());
  let (race_path, race_target) = (path.clone(), sentinel.clone());
  let (url, server) = serve("200 OK", move || std::os::unix::fs::symlink(race_target, race_path).unwrap());
  backend.save_url_new(url, path.clone());
  assert_failed(&backend);
  server.join().unwrap();
  assert_eq!(std::fs::read(sentinel).unwrap(), b"sentinel");
  assert!(std::fs::symlink_metadata(path).unwrap().file_type().is_symlink());
}

#[cfg(unix)]
#[test]
fn a_batch_download_refuses_a_dangling_symlink() {
  let dir = tempfile::tempdir().unwrap();
  let path = dir.path().join("artcraft_abc.png");
  let missing = dir.path().join("missing.png");
  std::os::unix::fs::symlink(&missing, &path).unwrap();
  let backend = backend(dir.path());
  let (url, server) = serve("200 OK", || {});
  backend.save_url_new(url, path.clone());
  assert_failed(&backend);
  server.join().unwrap();
  assert!(!missing.exists());
  assert_eq!(std::fs::read_link(path).unwrap(), missing);
}

#[test]
fn a_confirmed_single_file_save_still_replaces_the_file() {
  let dir = tempfile::tempdir().unwrap();
  let path = dir.path().join("artcraft_abc.png");
  std::fs::write(&path, b"old").unwrap();
  let backend = backend(dir.path());
  let (url, server) = serve("200 OK", || {});
  backend.save_url(url, path.clone());
  assert!(matches!(backend.rx.recv_timeout(TIMEOUT).unwrap(), Event::Saved(saved) if saved == path));
  server.join().unwrap();
  assert_eq!(std::fs::read(path).unwrap(), BODY);
}

#[test]
fn an_http_error_reports_failure_without_creating_a_file() {
  let dir = tempfile::tempdir().unwrap();
  let path = dir.path().join("artcraft_abc.png");
  let backend = backend(dir.path());
  let (url, server) = serve("500 Internal Server Error", || {});
  backend.save_url_new(url, path.clone());
  assert_failed(&backend);
  server.join().unwrap();
  assert!(!path.exists());
}

// Construct only download plumbing: never load the user's session or start a provider worker.
fn backend(root: &Path) -> Backend {
  let (tx, rx) = channel();
  Backend { rt: tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap(), ctx: egui::Context::default(), tx, rx, host: ApiHost::Localhost { port: 1 }, root: root.to_path_buf(), creds: Arc::new(RwLock::new(None)), http: reqwest::Client::builder().no_proxy().timeout(TIMEOUT).build().unwrap(), device_login_cancel: Arc::new(AtomicBool::new(false)), accounts: ProviderAccounts::load(root), jobs: JobStore::load(root) }
}

fn serve(status: &'static str, before_response: impl FnOnce() + Send + 'static) -> (String, thread::JoinHandle<()>) {
  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  listener.set_nonblocking(true).unwrap();
  let url = format!("http://{}/image.png", listener.local_addr().unwrap());
  let server = thread::spawn(move || {
    let start = Instant::now();
    let mut stream = loop {
      match listener.accept() {
        Ok((stream, _)) => break stream,
        Err(err) if err.kind() == std::io::ErrorKind::WouldBlock && start.elapsed() < TIMEOUT => thread::sleep(Duration::from_millis(5)),
        Err(err) => panic!("download did not connect: {err}"),
      }
    };
    stream.set_read_timeout(Some(TIMEOUT)).unwrap();
    let mut request = Vec::new();
    while !request.ends_with(b"\r\n\r\n") {
      let mut byte = [0];
      stream.read_exact(&mut byte).unwrap();
      request.push(byte[0]);
    }
    before_response();
    write!(stream, "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", BODY.len()).unwrap();
    stream.write_all(BODY).unwrap();
  });
  (url, server)
}

fn assert_failed(backend: &Backend) {
  match backend.rx.recv_timeout(TIMEOUT).unwrap() {
    Event::Toast { error: true, message } => assert!(message.starts_with("File download failed: "), "{message}"),
    _ => panic!("expected a download failure toast"),
  }
  assert!(backend.rx.try_recv().is_err(), "a failed download must not report success");
}

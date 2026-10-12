//! Bounded, nonblocking diagnostics. Sensitive payloads never enter this API.
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    fs::{self, File, OpenOptions},
    io::{BufWriter, Write},
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};
const MAX_BYTES: u64 = 1024 * 1024;
const MAX_RECENT: usize = 256;
struct Writer {
    sender: mpsc::SyncSender<String>,
}
#[derive(Default)]
pub(crate) struct Diagnostics {
    writer: Mutex<Option<Writer>>,
    path: Mutex<Option<PathBuf>>,
    writer_error: Arc<Mutex<Option<String>>>,
    recent: Mutex<VecDeque<String>>,
}
impl Diagnostics {
    pub fn configure(&self, path: PathBuf) -> Result<(), String> {
        let mut writer = self.writer.lock().map_err(|_| "日志锁错误")?;
        if writer.is_some() {
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        let (sender, receiver) = mpsc::sync_channel::<String>(256);
        let worker_path = path.clone();
        let writer_error = self.writer_error.clone();
        thread::Builder::new()
            .name("sonahop-log".into())
            .spawn(move || {
                if let Err(error) = write_loop(file, worker_path, receiver) {
                    if let Ok(mut state) = writer_error.lock() {
                        *state = Some(error.to_string());
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        *self.path.lock().map_err(|_| "日志路径锁错误")? = Some(path);
        *writer = Some(Writer { sender });
        Ok(())
    }
    pub fn log(&self, event: &str, fields: Value) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let mut line = json!({"timeUnixMs":timestamp,"event":event,"fields":fields}).to_string();
        // Caller supplies structured bounded metadata, never raw control/media bodies.
        if line.len() > 8192 {
            return;
        }
        line.push('\n');
        if let Ok(mut recent) = self.recent.lock() {
            if recent.len() >= MAX_RECENT {
                recent.pop_front();
            }
            recent.push_back(line.clone());
        }
        if let Ok(writer) = self.writer.lock() {
            if let Some(writer) = writer.as_ref() {
                let _ = writer.sender.try_send(line);
            }
        }
    }
    pub fn path(&self) -> Option<String> {
        self.path
            .lock()
            .ok()?
            .as_ref()
            .map(|p| p.display().to_string())
    }
    pub fn error(&self) -> Option<String> {
        self.writer_error.lock().ok()?.clone()
    }
    pub fn recent(&self) -> String {
        self.recent
            .lock()
            .map(|lines| lines.iter().cloned().collect())
            .unwrap_or_default()
    }
}
fn write_loop(file: File, path: PathBuf, receiver: mpsc::Receiver<String>) -> std::io::Result<()> {
    let mut size = file.metadata()?.len();
    let mut writer = BufWriter::new(file);
    while let Ok(line) = receiver.recv() {
        if size + line.len() as u64 > MAX_BYTES {
            writer.flush()?;
            drop(writer);
            let backup = path.with_extension("jsonl.1");
            match fs::remove_file(&backup) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
            fs::rename(&path, &backup)?;
            writer = BufWriter::new(OpenOptions::new().create(true).append(true).open(&path)?);
            size = 0;
        }
        writer.write_all(line.as_bytes())?;
        writer.flush()?;
        size += line.len() as u64;
    }
    writer.flush()
}

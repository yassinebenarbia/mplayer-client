use std::fs::OpenOptions;
use std::io::prelude::*;

pub trait StringFeatures {
    /// insert [content] if the requested [String] is empty
    fn insert_if_empty(&mut self, content: &str);
}

#[allow(dead_code)]
/// logs _data_ to a _file_ in a incremantive manner
/// 
/// Panics: 
/// - failed to create or write to the file
pub fn log(data: &str, filename: &str) -> std::io::Result<std::fs::File>{
    let mut f = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(filename)?;
    writeln!(f, "{}", data)?;
    return std::io::Result::Ok(f)
}

impl StringFeatures for String {
    fn insert_if_empty(&mut self, content: &str) {
       if self.is_empty() {
           self.push_str(content);
       } 
    }
}


#[derive(PartialEq, Eq, Debug, Ord, PartialOrd, Clone, serde::Deserialize, serde::Serialize)]
#[derive(zbus::zvariant::Type)]
pub struct RunStatus{
    error_messge: String,
    status_type: StatusOption,
}

#[derive(PartialEq, Eq, Debug, Ord, PartialOrd, Clone, serde::Deserialize, serde::Serialize)]
#[derive(zbus::zvariant::Type)]
pub enum StatusOption {
    Ok,
    OutOfRange,
    CoudntPreformAction,
    CoudntGetSHandler,
    CoudntSeek,
    CoudntPauseManager,
    CoudntPauseHandler,
    CoudntResumeManager,
    CoudntResumeHandler,
    WrongPath,
    CoudntReadMusicData,
}

impl RunStatus {
    fn new(msg: String, status: StatusOption) -> Self {
        Self {
            error_messge: msg, status_type: status
        }
    }

    fn ok() -> Self {
        Self::new(String::from(""), StatusOption::Ok)
    }

    fn is_ok(&self) -> bool {
        match self.status_type {
            StatusOption::Ok => true,
            _ => false
        }
    }

    fn handler_errror() -> Self {
        return RunStatus::new(
            format!("coudn't get stream handler!"),
            StatusOption::CoudntGetSHandler
        )
    }
}

pub struct FPSController {
    fps: u32,
    instant: Instant,
    frame_window: f64,
}

impl Default for FPSController {
    fn default() -> Self {
        Self {
            fps: 30,
            instant: Instant::now(),
            frame_window: 1.0/30.0,
        }
    }
}

impl FPSController {
    pub fn new(fps: u32) -> Self {
        Self {
            fps, instant: Instant::now(), frame_window: 1.0/fps as f64,
        }
    }

    pub fn check_fps(&mut self) -> bool{
        let now = Instant::now();
        if (now - self.instant).as_secs_f64() > self.frame_window  {
            self.register_instant();
            return true;
        }
        return false;
    }

    fn register_instant(&mut self) {
        self.instant = Instant::now();
    }

    pub fn change_fps(&mut self, fps: u32) {
        self.fps = fps;
        self.frame_window = 1.0 / fps as f64;
    }
}

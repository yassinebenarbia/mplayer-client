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

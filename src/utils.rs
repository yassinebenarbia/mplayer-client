use std::fs::OpenOptions;
use std::io::prelude::*;

/// logs _data_ to a _file_ in a incremantive manner
/// 
/// Panics: 
/// - if file does not exist
pub fn log(data: &str, filename: &str) -> std::io::Result<std::fs::File>{
    let mut f = OpenOptions::new()
        .write(true)
        .append(true)
        .open(filename)?;
    writeln!(f, "{}", data)?;
    return std::io::Result::Ok(f)
}

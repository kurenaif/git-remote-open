use std::path::Path;
use std::process::{Command, ExitStatus};

// if status_code is not 0 return Err
fn status_2_result(status: &ExitStatus, message: &'static str) -> Result<i32, &'static str> {
    let status_code = status.code().unwrap();
    match status_code {
        0 => Ok(status_code),
        _ => Err(message)
    }
}

// get the url of the remote of the repository to which the path belongs.
fn get_remote_url(path: &Path) -> Result<String, &str> {
    let process = Command::new("git")
    .current_dir(path)
    .arg("config")
    .arg("remote.origin.url")
    .output()
    .expect("failed to get url");

    try!(status_2_result(&process.status, "failed to run \"git config remote.origin.url\""));
    
    let mut res = process.stdout;
    res.pop();// remove \n

    Ok(String::from_utf8_lossy(&res).to_string())
}

fn main() {
    println!("{}", get_remote_url(Path::new(".")).unwrap());
}

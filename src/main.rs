extern crate regex;
extern crate open;

use regex::{RegexSet, Regex};

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

// convert git remote url to https url
fn create_https_url(url: &str) -> Result<String, &str> {
    let regexes = [
        r"git@github.com:(.+)", // 0: ssh github
        r"https://github.com/(.+)", // 1: https github
    ];

    let set = RegexSet::new(
        &regexes
    ).unwrap();

    let matches: Vec<_> = set.matches(url).into_iter().collect();
    if matches.len() != 1 {
        return Err("Multiple url matches.");
    }

    let re = Regex::new(regexes[0]).unwrap();
    let caps = re.captures(url).unwrap();

    match matches[0] {
        0 | 1 => { // github
            let res = "https://github.com/".to_owned() + &caps[1].to_string();
            Ok(res)
        },
        _ => {
            panic!("regex matched but regex is not match.(This message should not come out)")
        }
    }
}

fn main() {
    let url = create_https_url(&get_remote_url(Path::new(".")).unwrap()).unwrap();
    open::that(&url);
}

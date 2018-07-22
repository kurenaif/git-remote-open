extern crate regex;
extern crate open;
extern crate clap;

use regex::{RegexSet, Regex};

use std::path::Path;
use std::process::{Command, ExitStatus};
use clap::{App, Arg};

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

// get the root path (which you run `git init`)
fn get_local_root_path(path: &Path) -> Result<String, &str> {
    let process = Command::new("git")
    .current_dir(path)
    .arg("rev-parse")
    .arg("--show-toplevel")
    .output()
    .expect("failed to get root path");

    try!(status_2_result(&process.status, "failed to run \"git rev-parse --show-toplevel\""));
    
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

    let re = Regex::new(regexes[matches[0]]).unwrap();
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
    let matches = App::new("auto_wmake")
    .version("0.1")
    .author("kurenaif <antyobido@gmail.com>")
    .about("open github page")
    .arg(Arg::with_name("path")
        .help("Path of the git repository where you want to open github.")
        .index(1))
    .get_matches();

    let path = matches.value_of("path").unwrap_or(".");

    let remote_url = match get_remote_url(Path::new(path)) {
        Ok(url) => url,
        Err(message) => {eprintln!("{}", message); std::process::exit(1)}
    };

    let host = match create_https_url(&remote_url) {
        Ok(url) => url,
        Err(message) => {eprintln!("{}", message); std::process::exit(1)}
    };

    println!("{}", host);
}

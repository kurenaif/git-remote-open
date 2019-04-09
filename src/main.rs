extern crate regex;
extern crate open;
extern crate clap;

use regex::{RegexSet, Regex};

use std::fs;
use std::path::{Path};
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
fn get_local_root_path_string(path: &Path) -> Result<String, &str> {
    let process = Command::new("git")
    .current_dir(path)
    .arg("rev-parse")
    .arg("--show-toplevel")
    .output()
    .expect("failed to get root path");

    try!(status_2_result(&process.status, "failed to run \"git rev-parse --show-toplevel\""));
    
    let mut abspath_vec = process.stdout;
    abspath_vec.pop();// remove \n

   let abspath_string = String::from_utf8_lossy(&abspath_vec).to_string();

   Ok(abspath_string)
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
            let connected_str = "https://github.com/".to_owned() + &caps[1].to_string();
            let re = Regex::new(r"\.git$").unwrap();
            let res = re.replace_all(&connected_str, "");
            Ok(res.to_string())
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
    .arg(Arg::with_name("root")
        .short("r")
        .long("root")
        .help("open root page regardless of argument \"path\""))
    .arg(Arg::with_name("silent")
        .short("s")
        .long("slient")
        .help("not open browser (only url standard output)"))
    .arg(Arg::with_name("line")
        .short("l")
        .long("line")
        .value_name("N")
        .help("open line number")
        .takes_value(true))
    .get_matches();

    let path = fs::canonicalize(matches.value_of("path").unwrap_or(".")).unwrap();

    let path_dir =
    if path.is_dir() {
        path.as_path()
    } else {
        match path.parent() {
            Some(parent) => parent,
            None => {
                eprintln!("error: {}'s parent is not found", path.to_str().unwrap());
                std::process::exit(1);
            }
        }
    };

    let remote_url = match get_remote_url(&path_dir) {
        Ok(url) => url,
        Err(message) => {eprintln!("{}", message); std::process::exit(1)}
    };

    let host = match create_https_url(&remote_url) {
        Ok(url) => url,
        Err(message) => {eprintln!("{}", message); std::process::exit(1)}
    };

    let root_path = match get_local_root_path_string(&path_dir){
        Ok(path) => path,
        Err(message) => {eprintln!("{}", message); std::process::exit(1)}
    };

    let ref_path = path.strip_prefix(root_path).unwrap();

    let root_path_str = ref_path.to_str().unwrap().to_string();

    let source_url = if root_path_str.is_empty() || matches.is_present("root") {
        host
    } else {
        host + "/tree/master/" + &root_path_str
    };

    let open_url = if matches.is_present("line") {
        source_url + "#L" + matches.value_of("line").unwrap()
    } else {
        source_url
    };

    println!("{}", open_url);

    if !matches.is_present("silent") {
        open::that(open_url);
    }
}

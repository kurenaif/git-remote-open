extern crate regex;
extern crate open;
extern crate clap;

use regex::{RegexSet, Regex};

use std::fs;
use std::path::{Path};
use std::process::{Command, ExitStatus};
use clap::{App, Arg};

enum Domain {
    Github,
}

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

// url parse to (domain, path)
fn parse_domain(url: &str) -> Result<(Domain, String), &str> {
    let regexes = [
        r"git@github.com:(.+)", // 0: ssh github
        r"https://github.com/(.+)", // 1: https github
    ];

    let set = RegexSet::new(
        &regexes
    ).unwrap();

    let matches: Vec<_> = set.matches(url).into_iter().collect();
    if matches.len() > 1 {
        return Err("Multiple url matches.");
    }
    else if matches.len() == 0 {
        return Err("domain not found");
    }

    let re = Regex::new(regexes[matches[0]]).unwrap();
    let caps = re.captures(url).unwrap();

    match matches[0] {
        0 | 1 => { // github
            Ok((Domain::Github, caps[1].to_string()))
        },
        _ => {
            panic!("regex matched but regex is not match.(This message should not come out)")
        }
    }
}

// convert git remote url to https url
fn create_https_url(url: &str) -> Result<String, &str> {
    let domain = parse_domain(url)?;

    match domain.0 {
        Domain::Github => { // github
            let connected_str = "https://github.com/".to_owned() + &domain.1;
            let re = Regex::new(r"\.git$").unwrap();
            let res = re.replace_all(&connected_str, "");
            Ok(res.to_string())
        },
        _ => {
            panic!("regex matched but regex is not match.(This message should not come out)")
        }
    }
}

fn line_number_to_string(domain: &Domain, line_option_str: &String) -> Result<String, &str> {
    match domain {
        Domain::Github => {
            if Regex::new(r"^\d+$").unwrap().is_match(line_option_str){
                Ok("#L".to_string() + line_option_str)
            } else if Regex::new(r"^\d+-\d+$").unwrap().is_match(line_option_str) {
                let line_numbers: Vec<&str> = line_option_str.split('-').collect();
                Ok("#L".to_string() + line_numbers[0] + "-#L" + line_numbers[1])
            } else {
                Err("error: line number's format is invalid")
            }
        },
        _ => {
            panic!("domain not found (but this message will be not appeared because it will have been to appeared at parse_domain function)")
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
        .value_name("N[-N]")
        .help("open line numbers: \"line_number\" or \"[line_start_number]-[line_end_number]\"")
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

    let domain = match parse_domain(&remote_url) {
        Ok(domain) => domain.0,
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
        match line_number_to_string(&domain, &matches.value_of("line").unwrap().to_string()) {
            Ok(line_str) => line_str,
            Err(message) => {eprintln!("{}", message); std::process::exit(1)}
        }
    } else {
        source_url
    };

    println!("{}", open_url);

    if !matches.is_present("silent") {
        open::that(open_url);
    }
}

extern crate clap;
extern crate open;
extern crate regex;

use regex::{Regex, RegexSet};

use clap::{App, Arg};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

enum Domain {
    Github,
}

/// some git command require dir's abs path.
/// so, process argumented file path to it's dir(parent) path.
/// note: file's parent dir always exists because repository root dir is exist.
fn get_abs_dir_path(path: &Path) -> Result<PathBuf, String> {
    let path = fs::canonicalize(path).unwrap();

    if path.is_dir() {
        Ok(path)
    } else {
        match path.parent() {
            Some(parent) => Ok(parent.to_path_buf()),
            None => Err(format!(
                "error: {}'s parent is not found",
                path.to_str().unwrap()
            )),
        }
    }
}

/// if status_code is not 0 return Err
fn status_2_result(status: ExitStatus, message: &'static str) -> Result<i32, &'static str> {
    let status_code = status.code().unwrap();
    match status_code {
        0 => Ok(status_code),
        _ => Err(message),
    }
}

/// get the url of the remote of the repository to which the path belongs.
fn get_remote_url(path: &Path) -> Result<String, &str> {
    let dir_path = get_abs_dir_path(path).unwrap();
    let process = Command::new("git")
        .current_dir(&dir_path)
        .arg("config")
        .arg("remote.origin.url")
        .output()
        .expect("failed to get url");

    try!(status_2_result(
        process.status,
        "failed to run \"git config remote.origin.url\""
    ));

    let mut res = process.stdout;
    res.pop(); // remove \n

    Ok(String::from_utf8_lossy(&res).to_string())
}

/// get the root path (which you run `git init`)
fn get_local_root_path_string(path: &Path) -> Result<String, String> {
    let dir_path = get_abs_dir_path(path).unwrap();

    let process = Command::new("git")
        .current_dir(&dir_path)
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
        .expect("failed to get root path");

    try!(status_2_result(
        process.status,
        "failed to run \"git rev-parse --show-toplevel\""
    ));

    let mut abspath_vec = process.stdout;
    abspath_vec.pop(); // remove \n

    let abspath_string = String::from_utf8_lossy(&abspath_vec).to_string();

    Ok(abspath_string)
}

/// url parse to (domain, path)
fn parse_domain(url: &str) -> Result<(Domain, String), &str> {
    let regexes = [
        r"git@github.com:(.+)",     // 0: ssh github
        r"https://github.com/(.+)", // 1: https github
    ];

    let set = RegexSet::new(&regexes).unwrap();

    let matches: Vec<_> = set.matches(url).into_iter().collect();
    if matches.len() > 1 {
        return Err("Multiple url matches.");
    } else if matches.is_empty() {
        return Err("domain not found");
    }

    let re = Regex::new(regexes[matches[0]]).unwrap();
    let caps = re.captures(url).unwrap();

    match matches[0] {
        0 | 1 => {
            // github
            Ok((Domain::Github, caps[1].to_string()))
        }
        _ => panic!("regex matched but regex is not match.(This message should not come out)"),
    }
}

/// convert git remote url to https url
fn create_https_url(url: &str) -> Result<String, &str> {
    let domain = parse_domain(url)?;

    match domain.0 {
        Domain::Github => {
            // github
            let mut connected_str = "https://github.com/".to_owned() + &domain.1;
            let root_url = if connected_str.ends_with(".git") {
                let len = connected_str.len();
                connected_str.truncate(len - 4);
                connected_str
            } else {
                connected_str
            };
            Ok(root_url)
        }
    }
}

///
/// Convert command line arguments passed in '-l' to strings appropriate for each domain
///
/// # Examples
/// ```
/// -l {n}-{m} => path/to/url/#L{n}-#L{m}
/// -l {n} => path/to/url/#L{n}
/// ```
fn line_number_to_string(domain: &Domain, line_option_str: &str) -> Result<String, String> {
    match domain {
        Domain::Github => {
            if Regex::new(r"^\d+$").unwrap().is_match(line_option_str) {
                Ok("#L".to_string() + line_option_str)
            } else if Regex::new(r"^\d+-\d+$").unwrap().is_match(line_option_str) {
                let line_numbers: Vec<&str> = line_option_str.split('-').collect();
                Ok("#L".to_string() + line_numbers[0] + "-L" + line_numbers[1])
            } else {
                Err("error: line number's format is invalid".to_string())
            }
        }
    }
}

fn get_current_branch_name(path: &Path) -> Result<String, &str> {
    let dir_path = get_abs_dir_path(path).unwrap();

    let process = Command::new("git")
        .current_dir(&dir_path)
        .arg("branch")
        .output()
        .expect("failed to get root path");

    status_2_result(process.status, "failed to run \"git branch\"")?;

    let branches = String::from_utf8_lossy(&process.stdout).to_string();
    for branch in branches.split('\n') {
        if branch.is_empty() {
            continue;
        }
        if branch.starts_with('*') {
            let mut branch_name_rev = branch.chars().rev().collect::<String>();
            branch_name_rev.pop();
            branch_name_rev.pop();
            let branch_name = branch_name_rev.chars().rev().collect::<String>();
            return Ok(branch_name);
        }
    }

    Err("error: current branch not found")
}

/// get open url
fn get_url(matches: &clap::ArgMatches) -> Result<String, String> {
    let path = Path::new(matches.value_of("path").unwrap_or("."));

    let remote_url = get_remote_url(&path)?;

    let domain = parse_domain(&remote_url)?.0;

    let host = create_https_url(&remote_url)?;

    let root_path = get_local_root_path_string(&path)?;

    let abs_path = fs::canonicalize(&path).unwrap();
    let ref_path = abs_path.strip_prefix(root_path).unwrap();

    let root_path_str = ref_path.to_str().unwrap().to_string();

    let branch_name = if matches.is_present("branch") {
        matches.value_of("branch").unwrap().to_string()
    } else {
        get_current_branch_name(path)?
    };

    let source_url = if root_path_str.is_empty() || matches.is_present("root") {
        host
    } else {
        host + "/tree/" + &branch_name + "/" + &root_path_str
    };

    if matches.is_present("line") {
        Ok(source_url.to_string()
            + &line_number_to_string(&domain, &matches.value_of("line").unwrap().to_string())?)
    } else {
        Ok(source_url)
    }
}

fn main() {
    let matches = App::new("git-remote-open")
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
    .arg(Arg::with_name("branch")
        .short("b")
        .long("branch")
        .value_name("branch name")
        .help("open with branch name (default: current branch)")
        .takes_value(true))
    .get_matches();

    let open_url = match get_url(&matches) {
        Ok(url) => url,
        Err(msg) => {
            eprintln!("error: {}", msg);
            ::std::process::exit(1);
        }
    };

    println!("{}", open_url);

    if !matches.is_present("silent") {
        let _ = open::that(open_url);
    }
}

extern crate ulid;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::fs::File;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use ulid::Ulid;

    struct TargetDir {
        dir_path: PathBuf,
    }

    impl TargetDir {
        pub fn new(remote_url: &str) -> TargetDir {
            let ulid = Ulid::new().to_string();
            let dir_path = Path::new("unit_test_dir").join(&ulid);
            fs::create_dir_all(&dir_path);

            let mut process = Command::new("git")
                .current_dir(&dir_path)
                .arg("init")
                .spawn()
                .expect("failed to git init");
            process.wait();

            let mut process = Command::new("git")
                .current_dir(&dir_path)
                .arg("config")
                .arg("--local")
                .arg("user.name")
                .arg("i_am_unit_test_man")
                .spawn()
                .expect("failed to git config local user.name");
            process.wait();

            let mut process = Command::new("git")
                .current_dir(&dir_path)
                .arg("config")
                .arg("--local")
                .arg("user.email")
                .arg("i_am_unit_test_man@unit_test_man.mail")
                .spawn()
                .expect("failed to git config local user.email");
            process.wait();

            let mut process = Command::new("git")
                .current_dir(&dir_path)
                .arg("commit")
                .arg("--allow-empty")
                .arg("-m")
                .arg("\"first commit\"")
                .spawn()
                .expect("failed to git init");
            process.wait();

            let mut process = Command::new("git")
                .current_dir(&dir_path)
                .arg("remote")
                .arg("add")
                .arg("origin")
                .arg(remote_url)
                .spawn()
                .expect("failed to add remote url");
            process.wait();

            TargetDir {
                dir_path: dir_path.to_path_buf(),
            }
        }

        pub fn create_file(&self, file_name: &Path) {
            File::create(&self.dir_path.join(file_name));
        }

        pub fn create_dir(&self, file_name: &Path) {
            fs::create_dir(&self.dir_path.join(file_name));
        }

        pub fn create_branch(&self, branch_name: &str) {
            let mut process = Command::new("git")
                .current_dir(&self.dir_path)
                .arg("branch")
                .arg(branch_name)
                .spawn()
                .expect("fail git branch command");
            process.wait();
        }

        pub fn checkout_branch(&self, branch_name: &str) {
            let mut process = Command::new("git")
                .current_dir(&self.dir_path)
                .arg("checkout")
                .arg(&branch_name)
                .spawn()
                .expect("fail git checkout command");
            process.wait();
        }
    }

    impl Drop for TargetDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.dir_path);
        }
    }

    #[test]
    fn github__ssh__get_remote_url() {
        let dummy_url = "git@github.com:kurenaif/git-remote-open-unit-test-dummy.git";
        let target_dir = TargetDir::new(&dummy_url);
        assert_eq!(get_remote_url(&target_dir.dir_path).unwrap(), dummy_url);
    }

    #[test]
    fn github__html__get_remote_url() {
        let dummy_url = "https://github.com/kurenaif/git-remote-open-unit-test-dummy.git";
        let target_dir = TargetDir::new(&dummy_url);
        assert_eq!(get_remote_url(&target_dir.dir_path).unwrap(), dummy_url);
    }

    #[test]
    fn get_git_init_dir__target_file() {
        let dummy_url = "https://github.com/kurenaif/git-remote-open-unit-test-dummy.git";
        let target_dir = TargetDir::new(&dummy_url);
        let target_filename = "hoge.txt";
        let target_path = target_dir.dir_path.join(&target_filename);
        target_dir.create_file(Path::new(target_filename));
        assert_eq!(
            Path::new(&get_local_root_path_string(&target_path).unwrap()),
            fs::canonicalize(&target_dir.dir_path).unwrap()
        );
    }

    #[test]
    fn get_git_init_dir__target_dir() {
        let dummy_url = "https://github.com/kurenaif/git-remote-open-unit-test-dummy.git";
        let target_dir = TargetDir::new(&dummy_url);
        let target_dirname = "hoge_dir";
        let target_path = target_dir.dir_path.join(&target_dirname);
        target_dir.create_dir(Path::new(target_dirname));
        assert_eq!(
            Path::new(&get_local_root_path_string(&target_path).unwrap()),
            fs::canonicalize(&target_dir.dir_path).unwrap()
        );
    }

    #[test]
    fn get__line_number_to_string__single_param() {
        assert_eq!(
            &line_number_to_string(&Domain::Github, "12").unwrap(),
            "#L12"
        );
    }

    #[test]
    fn get__line_number_to_string__range_param() {
        assert_eq!(
            &line_number_to_string(&Domain::Github, "12-34").unwrap(),
            "#L12-L34"
        );
    }

    #[test]
    fn current_branch_name__master() {
        let dummy_url = "https://github.com/kurenaif/git-remote-open-unit-test-dummy.git";
        let target_dir = TargetDir::new(&dummy_url);
        target_dir.create_branch("hogehoge_mogumogu");
        assert_eq!(
            get_current_branch_name(&target_dir.dir_path).unwrap(),
            "master"
        );
    }

    #[test]
    fn current_branch_name__new_branch() {
        let dummy_url = "https://github.com/kurenaif/git-remote-open-unit-test-dummy.git";
        let target_dir = TargetDir::new(&dummy_url);
        target_dir.create_branch("new_branch");
        target_dir.create_branch("dummy1");
        target_dir.create_branch("dummy2");
        target_dir.create_branch("dummy3");
        target_dir.checkout_branch("new_branch");
        assert_eq!(
            get_current_branch_name(&target_dir.dir_path).unwrap(),
            "new_branch"
        );
    }
}

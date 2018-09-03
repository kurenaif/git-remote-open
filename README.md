# git-remote-open

Github remote url open in browser.

## Requirement

[cargo(rust build tool and package manager)](https://rust-lang-ja.github.io/the-rust-programming-language-ja/1.6/book/getting-started.html)

## Usage

```
USAGE:
    git-remote-open [FLAGS] [path]

FLAGS:
    -h, --help       Prints help information
    -r, --root       open root page regardless of argument "path"
    -V, --version    Prints version information

ARGS:
    <path>    Path of the git repository where you want to open github.
```

### Example

#### open current dir

```
$ git-remote-open # open https://github.com/kurenaif/git-remote-open
```

#### open file in browser (master branch)

```
$ git-remote-open src/main.rs # => open https://github.com/kurenaif/git-remote-open/blob/master/src/main.rs
```

#### open path's root page

```
$ git-remote-open -r src/main.rs # => open https://github.com/kurenaif/git-remote-open
```


## Installation

```
cargo install --git https://github.com/kurenaif/git-remote-open
```

## Related tool

[typester/ghopen](https://github.com/typester/gh-open)
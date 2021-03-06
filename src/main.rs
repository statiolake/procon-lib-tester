use colored_print::color::ConsoleColor as CC;
use colored_print::colored_println;

use std::env;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};
use std::process::{Command, Stdio};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// テスト一つを表す構造体です。
#[derive(Debug)]
struct Test {
    /// テストする対象のライブラリ (*.hpp)
    library: PathBuf,

    /// そのライブラリをテストするプロジェクトのディレクトリ (*.test)
    project: PathBuf,
}

/// テスト結果を表す列挙体です。
#[derive(Debug)]
enum TestResult {
    Succeeded,
    Failed,
    NotFound,
}

impl Test {
    pub fn new(library: PathBuf) -> Test {
        let project = library.with_extension("test");
        Test { library, project }
    }

    pub fn judge(&self, force: bool, simple: bool) -> io::Result<TestResult> {
        if !self.project.exists() {
            return Ok(TestResult::NotFound);
        }

        let mut cmd = Command::new("procon-assistant");
        cmd.arg("--quiet");

        cmd.arg("run");

        if force {
            cmd.arg("--force");
        }

        cmd.current_dir(&self.project)
            .stdin(Stdio::null())
            .stdout(Stdio::null());

        if simple {
            cmd.stderr(Stdio::null());
        } else {
            cmd.stderr(Stdio::inherit());
        }

        let success = cmd.status()?.success();

        if success {
            Ok(TestResult::Succeeded)
        } else {
            Ok(TestResult::Failed)
        }
    }
}

impl TestResult {
    fn get_color(&self) -> CC {
        match *self {
            TestResult::Succeeded => CC::LightGreen,
            TestResult::Failed => CC::Red,
            TestResult::NotFound => CC::Yellow,
        }
    }
}

impl fmt::Display for TestResult {
    fn fmt(&self, b: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TestResult::Succeeded => write!(b, "SUCCESS"),
            TestResult::Failed => write!(b, "FAILURE"),
            TestResult::NotFound => write!(b, "MISSING"),
        }
    }
}

fn path_root_removed(path: &Path, root: &Path) -> String {
    let path = path.display().to_string();
    let root = {
        let mut root = root.display().to_string();
        root.push(MAIN_SEPARATOR);
        root
    };

    if path.starts_with(&root) {
        format!("{}", &path[root.len()..])
    } else {
        path
    }
}

fn main() -> Result<()> {
    let args = env::args().skip(1); // skip executable name
    let mut colorize = atty::is(atty::Stream::Stdout);
    let mut force = true;
    let mut simple = false;
    for arg in args {
        match &*arg {
            "--color=always" => colorize = true,
            "--color=none" => colorize = false,
            "--color=auto" => {}
            "--no-force" | "-n" => force = false,
            "--simple" | "-s" => simple = true,
            arg => return Err(format!("unknown command line argument: {}", arg).into()),
        }
    }

    let library_root = find_lib_root()?;
    println!("found library root at {}", library_root.display());

    let tests = enumerate_tests(&library_root)?;

    let (mut success, mut failure, mut notfound) = (0, 0, 0);
    for test in tests {
        let result = test.judge(force, simple)?;
        let color = result.get_color();

        colored_println! {
            colorize;
            CC::Reset, "[";
            color, "{}", result;
            CC::Reset, "] {}", path_root_removed(&test.library, &library_root);
        }

        match result {
            TestResult::Succeeded => success += 1,
            TestResult::Failed => failure += 1,
            TestResult::NotFound => notfound += 1,
        }
    }
    colored_println! {
        colorize;
        CC::Reset, "test finished. ";
        CC::Reset, "{} total, ", success + failure + notfound;
        TestResult::NotFound.get_color(), "{} ", notfound;
        CC::Reset, "skipped, ";
        TestResult::Succeeded.get_color(), "{} ", success;
        CC::Reset, "succeeded, ";
        TestResult::Failed.get_color(), "{} ", failure;
        CC::Reset, "failed.";
    };

    if failure != 0 {
        Err("some test failed.".into())
    } else {
        Ok(())
    }
}

/// ライブラリのルートディレクトリかどうか確認します。
fn check_root(path: &Path) -> bool {
    path.join("marker_lib_root").exists()
}

/// ライブラリのルートディレクトリを検索します。
fn find_lib_root() -> Result<PathBuf> {
    let mut curr = env::current_dir()?;
    // これはダミーのファイル名。
    // parent() で、カレントディレクトリから取得できるようにするため...
    curr.push("dummy");

    while let Some(parent) = curr.parent().map(Path::to_path_buf) {
        if check_root(&parent) {
            return Ok(parent);
        }
        curr = parent;
    }

    Err(From::from("failed to find library root."))
}

/// `target` 以下のテストファイルを全て列挙します。
fn enumerate_tests(target: &Path) -> io::Result<Vec<Test>> {
    let mut result = Vec::new();
    let entries = fs::read_dir(target)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|x| x.to_str()) == Some("hpp") {
            result.push(Test::new(path));
        } else if path.is_dir() {
            let children = enumerate_tests(&path)?.into_iter();
            result.extend(children);
        }
    }

    Ok(result)
}

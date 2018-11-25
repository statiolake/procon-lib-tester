use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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
}

fn main() -> Result<()> {
    let library_root = find_lib_root()?;
    println!("found library root at {}", library_root.display());

    let tests = enumerate_tests(&library_root)?;

    for test in tests {
        println!("found test: {:?}", test);
    }

    Ok(())
}

/// ライブラリのルートディレクトリかどうか確認します。
fn check_root(path: &Path) -> bool {
    path.join("marker_lib_root").exists()
}

/// ライブラリのルートディレクトリを検索します。
fn find_lib_root() -> Result<PathBuf> {
    let mut curr = env::current_dir().map_err(error_into_box)?;
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

/// `E: Error` を `Box<Error>` へ突っ込む関数です。通常こういう `box` 化は
/// `Result::map_error()` に渡すクロージャの形で `|err| Box::new(err)` とするこ
/// とが多いと思いますが、これ結構失敗します。というのも通常のケースでは `err`
/// は具体的な型の値なので、 `Box<SomeConcreteType>` の形に推論されてしまうから
/// です。その後たとえば `?` 演算子で早期リターンしようにも、
/// `Box<SomeConcreteType>` から`Box<dyn Error>` への型強制及び `From` 実装はな
/// いので、だいたいエラーになってしまいます。これを回避するには `Box::new(err)
/// as Box<dyn Error>` などとしなければならず、タイプ数が増えます。
///
/// そこでこの関数を噛ませることができるのです。これを使えばそもそも明示的に
/// `Box<dyn Error>` を返すようになっているので意図しない型に推論されることはあ
/// りません！やったね。
fn error_into_box<E: 'static + std::error::Error + Send + Sync>(
    error: E,
) -> Box<std::error::Error + Send + Sync> {
    Box::new(error)
}

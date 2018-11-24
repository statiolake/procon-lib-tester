use std::env;
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
    let library_root = find_lib_root()?;
    println!("found library root at {}", library_root.display());
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

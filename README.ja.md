# kforthc

[English README](README.md)

`kforthc` は Rust 製のコンパイラで、**kFORTH プログラム**を **LLVM IR** に変換し、`llc` と `clang` でオブジェクト/実行ファイル化します。

- FORTH 言語拡張は行いません。
- **kPascal の中間コード**（kFORTH）向けバックエンドとしても利用できます。
- 実運用パイプライン: **kPascal -> kFORTH -> LLVM -> オブジェクト/実行ファイル**
- `:` 定義など高水準ワードを実装しているため、サブセットFORTHコンパイラとしても利用可能です。

## Getting Started（PATH前提）

```bash
which kpascal
cargo build
./scripts/test_samples.sh
```

`which kpascal` でパスが出ない場合は、先に `kpascal` を `PATH` に追加してください。

## 言語仕様

現在合意している挙動（wrap/boolean/char幅/実行時トラップ/未初期化読み出し等）は `SPEC.md` を参照してください。

## 必要環境

- Rust (`cargo`)
- LLVM `llc`（または `llc-14`）
- `clang`
- `kpascal` が `PATH` 上にあること（Pascal連携/テストで必須）

## ビルドと実行（FORTH）

```bash
cargo build
./target/debug/kforthc example.fth out.ll
llc -filetype=obj out.ll -o out.o   # または llc-14
clang -no-pie out.o runtime/runtime.c -o a.out
./a.out
```

補助スクリプト:

```bash
./scripts/build.sh
```

## kPascal 連携

このリポジトリでは `kpascal` が `PATH` にある前提で実行します。

```bash
which kpascal
./scripts/test_kpascal_full.sh
```

## サンプル・テスト実行

- 通常/境界サンプル:
  ```bash
  ./scripts/test_samples.sh
  ```
- 異常系コンパイルテスト:
  ```bash
  ./scripts/test_negative_pascal.sh
  ```
- 実行時失敗テスト（`div/mod 0`）:
  ```bash
  ./scripts/test_runtime_failures.sh
  ```
- 再帰の回帰テスト:
  ```bash
  ./scripts/test_known_limitations.sh
  ```
- エラーメッセージスナップショット:
  ```bash
  ./scripts/test_error_messages_snapshot.sh
  ```
- 必須FORTHワード網羅テスト:
  ```bash
  ./scripts/test_required_words.sh
  ```

## ディレクトリ構成

- `src/main.rs`: コンパイラ本体（tokenize/parse/codegen）
- `runtime/runtime.c`: 生成コードが呼ぶランタイム
- `samples/`: Pascalサンプルと期待出力
- `scripts/`: ビルド/テストスクリプト
- `SPEC.md`: 言語仕様

## ライセンス

MIT。`LICENSE` を参照してください。

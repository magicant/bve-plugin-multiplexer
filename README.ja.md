# BVE Trainsim ATS Plugin Multiplexer

## 概要

Rust で書いた "DetailManager" のクローンです。
一つの車両で複数のプラグインが使えるようにします。

## オリジナルの DetailManager との違い

子プラグインのファイル名を書くテキストファイルのファイル名は `detailmodules.txt` に限定されておらず、DLL ファイル名に連動して変更できます。
このため 32 ビット版と 64 ビット版を同じフォルダーに入れることも可能です。

子プラグインの読み込みに失敗したとき、Plugin Multiplexer はエラーダイアログを表示します。

Plugin Multiplexer は、子プラグインの SetPower, SetBrake, SetReverser を過不足なく呼び出します。
DetailManager では、シナリオ開始直後にこれらが呼ばれなかったり過剰に呼ばれたりするケースがあります。

## ライセンス

[Unlicense](http://unlicense.org)

## 使用方法

1. [リリースページ](https://github.com/magicant/bve-plugin-multiplexer/releases)から bve-plugin-multiplexer-v1.zip をダウンロードしてください。ZIP ファイルの中に pluginmultiplexer32.dll と pluginmultiplexer64.dll が入っています。
1. 車両データのフォルダー内に pluginmultiplexer32.dll や pluginmultiplexer64.dll を移動します。
1. [車両ファイル](http://bvets.net/jp/edit/formats/vehicle/vehicle.html) (普通は Vehicle.txt のような名前が付けられているファイル) の `Ats32` と `Ats64` のところに pluginmultiplexer32.dll と pluginmultiplexer64.dll への相対パスをそれぞれ記入します。
    - 車両ファイルのバージョンが古い場合は `Ats` のところに pluginmultiplexer32.dll への相対パスのみ記入します。
    - 相対パスの基準は車両ファイルが置いてあるフォルダーです。
1. pluginmultiplexer32.dll と同じフォルダーに pluginmultiplexer32.txt というテキストファイルを作成し、読み込ませたい 32 ビット版プラグインへの相対パスを一行づつ記入します。
    - 相対パスの基準は pluginmultiplexer32.dll が置いてあるフォルダーです。
1. pluginmultiplexer64.dll と同じフォルダーに pluginmultiplexer64.txt というテキストファイルを作成し、読み込ませたい 64 ビット版プラグインへの相対パスを一行づつ記入します。
    - 相対パスの基準は pluginmultiplexer64.dll が置いてあるフォルダーです。

### 注意

- DLL ファイルの名前を変更した場合は、テキストファイルの名前も合わせて変更してください。拡張子以外の部分を揃える必要があります。
- 32 ビット版と 64 ビット版は同じフォルダーに入れることもできるし別々のフォルダーに分けることもできます。どちらにしても DLL ファイルに対応するテキストファイルは同じフォルダーに入れる必要があります。
- シナリオ開始時にエラーダイアログが表示された場合は、メッセージをよく読んで原因を修正してください。
    - ファイル名のスペルは正しいですか?
    - ファイル名を絶対パスで指定していませんか?
    - 相対パスの基準フォルダーは正しいですか?
    - 32 ビット版と 64 ビット版を取り違えていませんか?

## ビルド方法

以下の説明は、お使いの PC の OS が Windows であることを前提としています。

### Rust のインストール

下のリンクから Rust をダウンロードしてインストールします。

[Rust をインストール - Rust プログラミング言語](https://www.rust-lang.org/ja/tools/install)

### ツールチェーンの追加

コマンドラインで以下のコマンドを入力してください。

```plaintext
rustup target add i686-pc-windows-msvc
rustup target add x86_64-pc-windows-msvc
```

### ビルド

32bit 版の DLL は、以下のコマンドでコンパイルできます。

```plaintext
cargo build --release --target="i686-pc-windows-msvc"
```

64bit 版の DLL は、以下のコマンドでコンパイルできます。

```plaintext
cargo build --release --target="x86_64-pc-windows-msvc"
```

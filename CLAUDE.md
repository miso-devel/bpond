# bpond

Procedural koi pond animation in the terminal. Braille sub-pixel rendering + chain-dynamics spine.

## Build & Run

```bash
cargo run                       # デバッグビルド → 実行
cargo run --release             # リリースビルド → 実行
cargo run --release -- --debug  # ヘッダー付き（速度情報等）
cargo watch -x run              # ファイル変更時に自動リビルド
RUST_BACKTRACE=1 cargo run      # バックトレース付き実行
```

## Development

```bash
cargo check                # コンパイルチェック
cargo fmt                  # コードフォーマット
cargo fmt --check          # フォーマット検証のみ（CIと同じ）
cargo clippy -- -D warnings  # clippy リント（警告もエラー扱い）
cargo test                 # テスト実行
cargo clean                # ビルド成果物削除
```

CI と同等のチェックを手元で回す:
```bash
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

## Architecture

```
src/
├── main.rs      # イベントループ + 描画 (水面/餌/ヘッダー)
├── canvas.rs    # Braille サブピクセルキャンバス (1セル = 2×4ドット)
├── food.rs      # 餌ペレット: ライフサイクル管理
├── koi.rs       # 鯉: チェーンダイナミクス脊椎、ステアリング、描画
└── pond.rs      # 池: 鯉+餌の状態管理、座標変換ヘルパー
```

### 技術的なポイント

- **チェーンダイナミクス**: 40セグメントのワールド座標チェーン。頭が前進し、各セグメントが前のセグメントを追従。旋回時に体が自然にC字/S字に曲がる
- **Braille レンダリング**: Unicode braille (U+2800) で1セルあたり2×4=8サブピクセル。通常の8倍の解像度
- **均一スケール**: sx=sy にすることで heading によるサイズ変化を防止
- **生物力学ヒレ**: 角度ベースの開閉 (rest + amp × sin(ωt + phase))、左右交互

### 変更時の注意

- ブランチを切って作業し、承認されなければ捨てる
- ヒレのパラメータはワールド座標系（セル単位）で指定。スケール変更時は要調整
- `SEG_LEN` を変えると体長が変わり、`BODY_TOTAL` を通じて体幅・ヒレサイズも連動

## Key Bindings

- マウスクリック — 餌を落とす（鯉が寄ってきて食べる）
- `↑` / `↓` — 速度調整
- `q` / `Esc` — 終了
- `--debug` フラグ — ヘッダー表示（速度情報等）

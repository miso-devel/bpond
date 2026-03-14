# terminal-zoo

ASCII動物をターミナル上でぬるぬるアニメーションさせるCLIツール。

## Build & Run

```bash
make run       # デバッグビルド → 実行
make debug     # RUST_BACKTRACE=1 付きで実行
make release   # リリースビルド（最適化あり）
make run-release # リリースバイナリ実行
```

## Development

```bash
make check     # コンパイルチェックのみ（高速）
make fmt       # コードフォーマット
make lint      # clippy でリント
make test      # テスト実行
make ci        # fmt + lint + check + test（CI用）
make clean     # ビルド成果物削除
```

## Architecture

- `src/main.rs` — 全ロジックを含むシングルファイル構成
- `crossterm` — ターミナル制御（alternate screen, raw mode, cursor操作）
- `rand` — 動物の初期位置・速度のランダム化

## Key Bindings（実行中）

- `a` — 動物を追加
- `d` — 動物を削除
- `q` / `Esc` — 終了

## Notes

- フレームレートは30FPS固定
- 動物は壁で跳ね返る物理シミュレーション
- edition は 2021 を使用

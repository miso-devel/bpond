# terminal-zoo

リッチなASCII動物アニメーションをターミナル上で表示するCLIツール。
ratatui ベースで、グラデーション・パーティクル・星空背景付き。

## Build & Run

```bash
make run          # デバッグビルド → 実行
make debug        # RUST_BACKTRACE=1 付きで実行
make release      # リリースビルド（最適化あり）
make run-release  # リリースバイナリ実行
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
- `ratatui` — TUIフレームワーク（レイアウト、ウィジェット、バッファ直接描画）
- `crossterm` — ターミナルイベント制御
- `color-eyre` — エラーハンドリング
- `rand` — ランダム化

## Visual Features

- 背景グラデーション（上から下へ暗い紫〜紺）
- 瞬く星空背景
- 動物ごとの固有カラーグラデーション + シマー効果
- ゴースト残像トレイル
- バウンス時のパーティクルエフェクト
- 常時発生するアンビエントパーティクル
- フッターのレインボーグラデーションバー

## Key Bindings（実行中）

- `a` / `Space` — 動物を追加
- `d` / `Backspace` — 動物を削除
- `r` — パーティクルバースト
- `q` / `Esc` — 終了

## Notes

- フレームレート: ~30FPS（33ms tick）
- 動物は壁で跳ね返る物理シミュレーション + 波動モーション
- edition は 2021 を使用

# terminal-zoo

Ghosttyスタイルの大型ASCIIアート動物がぬるぬる浮遊するTUI。
ratatui + 60fps + サイン波合成によるオーガニックな動き。

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
make ci        # fmt + lint + check + test
make clean     # ビルド成果物削除
```

## Architecture

- `src/main.rs` — シングルファイル構成
- `ratatui` — TUIフレームワーク（バッファ直接描画）
- `crossterm` — ターミナルイベント制御
- `color-eyre` — エラーハンドリング
- `rand` — パーティクル生成

## Visual Features

- 大型ASCIIアート（25-30行）× 4種類（Cat, Dog, Fish, Rabbit）
- 文字密度に応じた輝度マッピング（@=最明, .=最暗）
- 3ストップカラーグラデーション（上→中→下）
- 複数サイン波合成による滑らかな浮遊モーション
- 行ごとの波うねりエフェクト
- 呼吸アニメーション（目の開閉）
- パルスグロー効果
- 瞬く星空背景
- アンビエントパーティクル
- レインボーフッター

## Key Bindings

- `←` / `→` / `n` / `p` — 動物を切り替え
- `q` / `Esc` — 終了

## Notes

- 60fps（16ms tick）
- 位置はサイン波合成で算出（バウンスなし）
- edition 2021

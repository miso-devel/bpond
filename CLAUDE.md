# terminal-zoo

可愛いASCIIアート動物がぬるぬる浮遊するTUI。
ratatui + 60fps + サイン波合成による滑らかな動き。

## Build & Run

```bash
make run          # デバッグビルド → 実行
make debug        # RUST_BACKTRACE=1 付きで実行
make release      # リリースビルド（最適化あり）
make run-release  # リリースバイナリ実行
```

## Development

```bash
make check     # コンパイルチェックのみ
make fmt       # コードフォーマット
make lint      # clippy でリント
make test      # テスト実行
make ci        # fmt + lint + check + test
make clean     # ビルド成果物削除
```

## Architecture

```
src/
├── main.rs              # App構造体、描画、イベントループ
└── animals/
    ├── mod.rs           # AnimalDef 構造体 + ANIMAL_DEFS 一覧
    ├── cat.rs           # 猫のASCIIアート + 色定義
    ├── dog.rs           # 犬
    ├── fish.rs          # 魚
    └── rabbit.rs        # うさぎ
```

- `ratatui` — TUIフレームワーク（バッファ直接描画）
- `crossterm` — ターミナルイベント制御
- `color-eyre` — エラーハンドリング
- `rand` — パーティクル生成

### 動物の追加方法

1. `src/animals/` に新しいファイル（例: `penguin.rs`）を作成
2. `ART_A`, `ART_B`（瞬き用）、`DEF` を定義
3. `src/animals/mod.rs` に `mod penguin;` と `ANIMAL_DEFS` への追加

## Key Bindings

- `←` / `→` / `n` / `p` — 動物を切り替え
- `q` / `Esc` — 終了

## Notes

- 60fps（16ms tick）
- 複数サイン波合成によるオーガニックな浮遊
- 背景なし（プレーン暗色）
- edition 2021

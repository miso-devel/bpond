# terminal-zoo

ASCIIアート動物がぬるぬる浮遊するTUI。ratatui + 60fps。

## Build & Run

```bash
make run          # デバッグビルド → 実行
make debug        # RUST_BACKTRACE=1 付きで実行
make release      # リリースビルド（最適化あり）
make run-release  # リリースバイナリ実行
```

## Development

```bash
make check     # コンパイルチェック
make fmt       # コードフォーマット
make lint      # clippy リント
make test      # テスト実行
make ci        # fmt + lint + check + test
make clean     # ビルド成果物削除
```

## Architecture

```
src/
├── main.rs              # App構造体、描画ループ、文字密度マッピング
└── animals/
    ├── mod.rs           # AnimalDef 構造体 + ANIMAL_DEFS 一覧
    ├── cat.rs
    ├── dog.rs
    ├── fish.rs
    └── rabbit.rs
```

### 動物の追加方法

1. `src/animals/` に新ファイル作成
2. `ART_A`(通常), `ART_B`(瞬き), `DEF` を定義
3. `mod.rs` に追加

### ASCIIアートの文字密度ルール

文字 → 輝度: `@`=100%, `$`=90%, `%`=78%, `*`=60%, `=`=50%, `+`=40%, `·`=12%

## Key Bindings

- `←` / `→` / `n` / `p` — 動物切り替え
- `q` / `Esc` — 終了

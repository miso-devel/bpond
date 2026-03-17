# mini-pond

Procedural koi pond animation in the terminal. Braille sub-pixel rendering + chain-dynamics spine.

## Build & Run

```bash
make run          # デバッグビルド → 実行
make watch        # ファイル変更時に自動リビルド (cargo-watch)
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
├── main.rs      # イベントループ、水面描画、ヘッダー
├── canvas.rs    # Braille サブピクセルキャンバス (1セル = 2×4ドット)
└── koi.rs       # 鯉: チェーンダイナミクス脊椎、体/ヒレ/尾の描画
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

- `↑` / `↓` — 速度調整
- `q` / `Esc` — 終了

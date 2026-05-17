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
├── main.rs           # イベントループ + 描画 (水面/餌/ヘッダー)
├── canvas.rs         # Braille サブピクセルキャンバス (1セル = 2×4ドット)
├── food.rs           # 餌ペレット: ライフサイクル管理
├── koi.rs            # 鯉: 構造体・定数・公開 API
├── koi/physics.rs    # ステアリング、体波 (animate_body)、Boids、サブステップ更新
├── koi/draw.rs       # 体・尾・ヒレ・目・burst 連動描画
├── pond.rs           # 池: 鯉+餌の状態管理、座標変換ヘルパー
├── ripple.rs         # 拡大する波紋
├── bubble.rs         # 上昇する泡
├── rain.rs           # 雨システム
└── rng.rs            # 共有疑似乱数
```

### 技術的なポイント

- **チェーンダイナミクス**: 40 セグメントのワールド座標チェーン。頭が前進し、各セグメントが前のセグメントを追従。旋回時に体が自然に C 字/S 字に曲がる
- **進行波 (animate_body)**: 頭→尾の位相遅延付きカーブをセグメントごとに適用。これが**尾鰭の動き**の正体。breath で振幅変調、burst でスケール
- **サブステップ積分**: 1 フレームを `SUBSTEPS = 3` に分割して微分方程式を積分。高速旋回や急加速でも滑らか
- **Boids スクーリング**: separation / alignment / cohesion の 3 力を `NEIGHBOR_RADIUS` 内の他鯉に対して適用。アイドル中のみ。重みは控えめで「群れ感を出す」程度
- **好奇心連鎖**: 近隣の鯉が餌に向いていると、自分も target_turn をその餌方向に寄せる。1 匹がリード → 周りが追従
- **Braille レンダリング**: Unicode braille (U+2800) で 1 セルあたり 2×4=8 サブピクセル。通常の 8 倍の解像度
- **均一スケール**: sx=sy にすることで heading によるサイズ変化を防止
- **生物力学ヒレ**: 角度ベースの開閉 (rest + amp × sin(ωt + phase))、左右交互。胸ビレは旋回時に内側が大きく開く (asymmetric brake)
- **burst 連動描画**: 現在の推進力 (`self.burst`) を描画時に参照して、ヒレ振幅・尾の広がりをスケール

### 変更時の注意

- ブランチを切って作業し、承認されなければ捨てる
- ヒレのパラメータはワールド座標系（セル単位）で指定。スケール変更時は要調整
- `SEG_LEN` を変えると体長が変わり、`BODY_TOTAL` を通じて体幅・ヒレサイズも連動
- `Koi::update` のシグネチャは `(dt, t, w, h, foods, others, my_idx)`。`others` は他鯉のスナップショット `(x, y, heading)`、`my_idx` で自分を除外する
- `Pond::update` は毎フレーム `Vec<(f64, f64, f64)>` で全鯉のスナップショットを集めて各鯉に渡す（borrow checker 対策）

## Key Bindings

- 左クリック — 餌を落とす（鯉が寄ってきて食べる）
- 右クリック — 近くの鯉を驚かせる（散逃 → 戻ってくる）
- `f` — ランダム位置に餌を落とす（マウス不要）
- `+` / `=` — 鯉を 1 匹追加
- `-` — 鯉を 1 匹削除
- `r` — 雨モード切替
- `↑` / `↓` — シミュレーション速度調整
- `q` / `Esc` — 終了
- `--debug` フラグ — ヘッダー表示（速度情報等）

## Releasing

crates.io 公開は tag push で自動化されています。

1. `Cargo.toml` の `version` を更新
2. `CHANGELOG.md` に新バージョンのエントリを追加
3. コミット & push
4. `git tag vX.Y.Z && git push origin vX.Y.Z`
5. `.github/workflows/release.yml` が `cargo publish` と GitHub Release を実行

前提: repo secrets に `CARGO_REGISTRY_TOKEN` が設定されていること（crates.io の Account Settings → API Tokens で発行）。

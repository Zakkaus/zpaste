# Zpaste

[English](README.md) | 繁體中文 | [简体中文](README.zh-CN.md)

Zpaste 是自架的 pastebin，用來閱讀 Markdown、程式碼與日誌，不論內容由人撰寫或由 AI 工具產生。它將取代 <https://paste.gentoozh.org> 上的 wastebin，讓該站的長篇內容容易閱讀。在同一網址修改 paste 與保留版本歷史已列入計畫。

Zpaste 以 Matthias Vogelgesang 的 [wastebin](https://github.com/matze/wastebin) 為基礎，沿用 wastebin 的儲存、加密、到期與閱後即焚程式碼。

## 狀態

Zpaste 仍在早期開發，以上游 wastebin 提交 `c0ec968` 為基礎，目前有三項改動：`/v1` API；由 `web/` 的網頁應用程式在瀏覽器繪製的閱讀頁（`GET /<id>[.<ext>]`，`/md/<id>` 會轉址到這裡）；伺服器執行檔名為 `zpaste`，日誌行的 target 也顯示為 `zpaste`。設定變數仍使用 `WASTEBIN_` 前綴，wastebin 原本的 HTTP API（`POST /`、`/raw`、`/dl`）照常可用。

## 建置

建置需要 Rust 1.95 以上版本與 C 編譯器，因為 SQLite 與 zstd 會從原始碼編譯。伺服器在建置時嵌入 `web/` 的網頁應用程式，所以要先建置它，這一步需要 Node.js 22.18 以上版本與 pnpm 11。在 repo 根目錄執行：

```sh
pnpm --dir web install --frozen-lockfile
pnpm --dir web build
cargo build --release
```

沒有 `web/dist` 時，`cargo build` 會發出警告，建置出的伺服器閱讀頁只會提示缺少網頁應用程式；兩種情況下測試都會通過。

伺服器輸出到 `target/release/zpaste`，資料庫工具輸出到 `target/release/wastebin-ctl`。

## 執行

以目前目錄下的資料庫檔案啟動伺服器：

```sh
WASTEBIN_DATABASE_PATH=state.db ./target/release/zpaste
```

伺服器監聽 `0.0.0.0:8088`。未設定 `WASTEBIN_DATABASE_PATH` 時，paste 只存在記憶體中，程式結束就會遺失。用瀏覽器開啟 <http://127.0.0.1:8088>，或在 shell 中建立 paste：

```sh
curl -s -H 'Content-Type: application/json' \
    --data '{"text": "# Hello", "extension": "md"}' \
    http://127.0.0.1:8088/
```

回應包含新 paste 的路徑，以及可以刪除它的 owner token：

```json
{"path":"/<id>.md","owner":"<token>"}
```

`curl http://127.0.0.1:8088/raw/<id>` 會回傳儲存的文字。設定變數、其餘 HTTP API 與 `wastebin-ctl` 的說明見 [wastebin 的 README](docs/upstream/WASTEBIN-README.md#configuration)。

## 容器映像

Dockerfile 會交叉編譯靜態連結 musl 的執行檔，設計為在 x86_64 主機上使用。在 repo 根目錄建置 x86_64 映像：

```sh
docker build --target amd64 -t zpaste .
```

映像建置會自行建置網頁應用程式。映像以使用者 10001 執行 `/app/zpaste`。

## 測試

```sh
cargo test
```

## 計畫

- 閱讀頁：渲染 Markdown 並附目錄，程式碼、日誌與 diff 各有適合的檢視方式。
- 在同一網址修改 paste，並保留版本歷史。
- 位於 `/v1` 下、帶版本號的 HTTP API，以及命令列用戶端。

工作計畫見 [docs/plan.zh-TW.md](docs/plan.zh-TW.md)。

## 上游文件

wastebin 的 README、變更紀錄與貢獻指南保留在 [docs/upstream](docs/upstream/README.md) 供參考。這些文件描述的是 wastebin，會隨 Zpaste 的修改而與現況不符。

## 授權

Zpaste 以 [MIT 授權](LICENSE)發佈。授權檔保留 Matthias Vogelgesang 對 wastebin 的版權聲明，並加上 Zakkaus 對 Zpaste 的版權聲明。

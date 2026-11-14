# Zpaste

[English](README.md) | 繁體中文 | [简体中文](README.zh-CN.md)

Zpaste 是自架的 pastebin，用來閱讀 Markdown、程式碼與日誌，不論內容由人撰寫或由 AI 工具產生。它將取代 <https://paste.gentoozh.org> 上的 wastebin，讓該站的長篇內容容易閱讀。在同一網址修改 paste 與保留版本歷史已列入計畫。

Zpaste 以 Matthias Vogelgesang 的 [wastebin](https://github.com/matze/wastebin) 為基礎，沿用 wastebin 的儲存、加密、到期與閱後即焚程式碼。

## 狀態

Zpaste 仍在早期開發。目前的行為與上游 wastebin 提交 `c0ec968` 相同，差別只有伺服器執行檔名為 `zpaste`，以及日誌行的 target 顯示為 `zpaste`。設定變數仍使用 `WASTEBIN_` 前綴，HTTP API 也與 wastebin 相同。

## 建置

建置需要 Rust 1.95 以上版本與 C 編譯器，因為 SQLite 與 zstd 會從原始碼編譯。在 repo 根目錄執行：

```sh
cargo build --release
```

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

映像以使用者 10001 執行 `/app/zpaste`。

## 測試

```sh
cargo test
```

## 計畫

- 閱讀頁：渲染 Markdown 並附目錄，程式碼、日誌與 diff 各有適合的檢視方式。
- 在同一網址修改 paste，並保留版本歷史。
- 位於 `/v1` 下、帶版本號的 HTTP API，以及命令列用戶端。

## 上游文件

wastebin 的 README、變更紀錄與貢獻指南保留在 [docs/upstream](docs/upstream/README.md) 供參考。這些文件描述的是 wastebin，會隨 Zpaste 的修改而與現況不符。

## 授權

Zpaste 以 [MIT 授權](LICENSE)發佈。授權檔保留 Matthias Vogelgesang 對 wastebin 的版權聲明，並加上 Zakkaus 對 Zpaste 的版權聲明。

> 本文件是 Zpaste 的工作計畫。Zakk 的決定記在第八節與文中標了日期的地方；架構決定的索引在 `docs/architecture.md` 的 Decisions 表，數字與門禁在 `REQUIREMENTS.md`。

# Zpaste 方案書（v12）

狀態：草案。專案名 **Zpaste**（repo `Zakkaus/zpaste`），以 wastebin（matze/wastebin，MIT）為基礎，前端換成 Astryx，API 改成 OpenStack 風格；網址仍是 paste.gentoozh.org
授權：MIT。`LICENSE` 保留上游 `Copyright 2022 Matthias Vogelgesang` 並加上我們的版權行；README 註明 based on wastebin 並附連結
原則：**重點是前端的可讀性、統一，以及共用元件好維護。** 只有一個服務、一個網址；和 gentoozh.org 的統一靠共用設計系統，不靠放在同一個網站。

## 一、要解決什麼

wastebin 和市面上的協作工具，可讀性都差。以 `/md/r8BaRKfkXfi.md`（rules kit，8 個檔案、1,646 行）為例：

- 包在區塊裡的 Markdown 不渲染：約一百個標題和全部表格都以原始碼顯示
- 沒有目錄，內文鎖在 78ch，寬表格放不下
- 原文 76 KB，產出的 HTML 有 754 KB（上色用的 inline `<span>` 有 12,612 個）
- 不能修改：要改只能重貼，重貼會產生新連結；在 shell 裡也沒有修改的方法
- API 零散：建立是 `POST /`、刪除是 `DELETE /{id}`、原文在 `/raw/{id}`，密碼放自訂標頭，回應格式各自不同
- 視覺與 gentoozh.org 不一致，兩套樣式各自維護

## 二、架構

```text
Zpaste（一個 repo、一個執行檔、一個網址 paste.gentoozh.org）
 ├─ 後端：Rust axum + SQLite（沿用上游的儲存、加密、到期、閱後即焚）
 │    ├─ /v1/...  新 API（第四節），前端、gzpaste 都只用它
 │    └─ 舊路由  POST /、/raw/<id>、DELETE /<id>：保留給 curl 一行指令，內部轉到同一套服務
 ├─ HTML 外殼由 Rust 輸出（每個閱讀頁請求）
 │    ├─ Open Graph：標題、類型、前幾行摘要（Telegram 預覽用）
 │    ├─ 一般 paste：內容與 ETag 以 JSON 嵌入頁面，前端不再抓第二次；<noscript> 顯示純文字
 │    ├─ 加密或閱後即焚：不嵌入內容、不放摘要，只放通用標題；不支援無 JS 閱讀
 │    └─ 跳脫：嵌入的 JSON 跳脫 < > & 與 U+2028/2029（避免內容裡的 </script> 提早結束區塊）；
 │       OG 屬性與 <noscript> 各用對應的 HTML 跳脫；啟動腳本是外部檔案，CSP 不開 inline
 ├─ 前端：Vite + React + Astryx + StyleX，建置產物嵌入執行檔
 │    ├─ /<id>[.<ext>]  閱讀頁（網址有副檔名就用它，否則用建立時存下的副檔名，都沒有就當純文字）
 │    ├─ /<id>/edit     編輯頁
 │    ├─ /<id>/history  版本頁
 │    └─ /              新建頁
 └─ 其他網址
      ├─ 舊的 /md/<id>.<ext> 302 到 /<id>.<ext>；/burn、/dl、/qr 保留
      ├─ 建立時的副檔名存進資料庫（新欄位，舊 paste 為空），建立後不能改
      └─ ?owner= 交換在產生 HTML 之前完成：寫入 cookie 後 302 到不帶 token 的網址，回應 no-store，日誌遮蔽 owner 參數
gentoozh.org
 ├─ /paste/  維持原本的說明頁，連到 paste.gentoozh.org 與 API 參考文件
 └─ 主題 token 發佈成有版本號的共用套件，兩邊各自鎖版本
```

- 上游的 askama 模板、CSS 與 JS 移除，HTML 路由改由「Rust 外殼 + 嵌入的前端」提供。
- CSP 在上游基礎上補 `connect-src 'self'`，其餘維持嚴格；開發時用 Vite 的 proxy 指向任何一個 wastebin 實例，不開 CORS。
- 驗證與 CSRF 依憑證種類決定（每個端點的細節見第四節的端點表）：
  - **不帶憑證**：可以建立 paste、讀取公開內容、帶密碼或確認閱後即焚取內容。匿名的 curl 與 gzpaste 照常可用。
  - **Bearer**：瀏覽器不會自動送出，沒有 CSRF 問題，不檢查 Origin。
  - **cookie**（網頁）：會自動送出，所以帶 cookie 的寫入請求必須有 `Origin`，且等於設定裡的公開網址（scheme、host、port 全比對，不信任轉發標頭）；沒有、`null` 或其他網域一律 403。
  - 每條規則都有測試，包括匿名的舊 `POST /`。
  - 舊版 gzpaste 用 cookie 刪除且不帶 Origin，P1 起會被拒絕；Zakk 已確認不需相容舊用戶端，新版 gzpaste 與 P1 同時發佈。
- 不允許搜尋引擎索引 paste（`noindex`）；說明頁照常索引。

### 前端怎麼取得內容

```text
打開 /<id>.<ext>
 ├─ 一般 paste ──▶ 內容已在頁面裡，直接渲染
 └─ 其他情況 ──▶ GET /v1/pastes/<id>（不會消耗內容）
      ├─ 404 ──▶ 不存在或已過期的畫面
      ├─ 加密 ──▶ 密碼畫面 ──▶ POST /v1/pastes/<id>/content 帶密碼
      │                       └─ 密碼錯誤 ──▶ 401（統一的錯誤 JSON）
      ├─ 閱後即焚 ──▶ 確認畫面 ──▶ 使用者按下才 POST /v1/pastes/<id>/content
      │                          └─ 不自動重試；失敗就告知已失效
      └─ 加密且閱後即焚 ──▶ 先輸入密碼，再按確認，兩步都完成才消耗
```

## 三、可讀性：閱讀頁依類型選渲染器

| 類型 | 渲染 |
|---|---|
| `.md` | Astryx `Markdown`：目錄（`Outline`）、標題錨點、GFM alerts、寬表格在容器內捲動且表頭固定、` ```markdown ` 區塊可切換渲染 |
| 程式碼 | `CodeBlock`：上色、行號、行錨點、複製 |
| 日誌與純文字 | 行號、切換換行 |
| `.diff`、`.patch` | 統一 diff |
| 圖與思維導圖（P1b） | mermaid：` ```mermaid ` 區塊與 `.mermaid` 檔，含流程圖、時序圖與 `mindmap`；`securityLevel: 'strict'` |
| | 思維導圖檢視：任何 `.md` 一鍵切換，由標題與清單階層產生；切換狀態放在網址（`?view=mindmap`） |
| | 兩者都按需載入：內容裡沒有 mermaid、使用者沒按切換，就不下載 |

### 思維導圖

markmap 或 `d3-flextree` 只負責版面計算；節點、連線、配色全部用 Astryx token 自己畫。

- 配色：第一層分支只用少數低飽和色，往下只變淺不換色；連線細、單色、弧度小。
- 文字：節點用內文字級，CJK 行高換行，節點有最大寬度。
- 寬螢幕：橫向導圖，預設展開兩層，有「適應畫面」。
- 窄螢幕：改成直向可摺疊樹，點一下展開下一層並顯示子節點數，不需要縮放。
- 點節點跳到原文段落，原文目錄也能跳回節點；鍵盤：方向鍵移動、Enter 展開、Esc 收合；可匯出 SVG、PNG。

### 共通

- 內容以純文字嵌入外殼，在瀏覽器端渲染，不再輸出 754 KB 的 HTML；mermaid 等重型渲染器只在需要時載入。
- 手機優先：目錄在手機上改成抽屜，320–1280 寬度都不出現整頁橫向捲動。
- 狀態：載入時顯示 skeleton；過期、不存在、需要密碼、閱後即焚確認、網路錯誤各有自己的畫面。
- 安全靠函式庫自己的設定，不自己寫 sanitizer：Astryx `Markdown` 轉成 React 元件、不執行原始 HTML，不安全的網址有內建處理；mermaid 用 strict；思維導圖節點由 React 繪製，文字自動跳脫。我們自己寫的只有 Rust 外殼嵌入 JSON 的跳脫，只為這一處寫一個 `</script>` 測試。
- 語系：zh-CN、zh-TW、en。主題：淺色、深色、跟隨系統。
- 缺少的元件功能用 `astryx gap-report` 回報，不複製元件原始碼來改。

延後，用到再做：日誌的 ANSI 顏色與跳到錯誤行、CSV/TSV 表格、左右對照 diff、多檔自動拆分頁。

## 四、API（OpenStack 風格，`/v1`）

### 身分

- 一個身分就是上游的一個 `uid`，owner token 是伺服器對它的簽章。
- 第一次建立時不帶憑證，伺服器產生新的身分，回應裡的 `owner_token` 只出現這一次。
- 之後建立時帶上 `Authorization: Bearer <owner_token>`，伺服器沿用同一個身分，所以「我的 paste」列得出全部。帶了但驗證失敗就回 401，不會偷偷換成新身分。
- 網頁用同網域 cookie 存同一個 token；`?owner=` 只用來把自己的身分帶到自己的另一台機器或瀏覽器。
- 權限範圍是整個身分，不是單一 paste，所以不做「分享編輯連結」。

### 端點表

| 方法與路徑 | 憑證 | 請求 | 成功 | 主要錯誤 |
|---|---|---|---|---|
| `GET /` | — | — | 新建頁（HTML） | — |
| `GET /v1` | — | — | `{"version": {"id": "v1", "status": "CURRENT"}}` | — |
| `POST /v1/pastes` | 可無；有就沿用身分 | `{"paste": {...}}`，見下 | 201，`Location`、`ETag` | 400、401、413、429 |
| `GET /v1/pastes?owner=me` | 必須 | `limit`（預設 20，最多 100）、`marker` | 200 | 400（marker 無效）、401 |
| `GET /v1/pastes/{id}` | — | — | 200，白名單欄位 | 404 |
| `PATCH /v1/pastes/{id}` | 必須 | `{"paste": {"title": ...}}`，可帶 `If-Match`；加密的 paste 帶 `X-Paste-Password` | 200，新 `ETag` | 401、403、404、409（閱後即焚）、412 |
| `DELETE /v1/pastes/{id}` | 必須 | — | 204 | 401、403、404 |
| `GET /v1/pastes/{id}/content` | — | — | 200 `text/plain`，`ETag` | 404；加密或閱後即焚回 409（`password_required`、`confirmation_required`），不消耗內容 |
| `POST /v1/pastes/{id}/content` | — | `{"password": ...}` 或 `{"confirm_burn": true}` | 200 `{"content": {"text", "version_id"}}`，`ETag` | 401（密碼錯誤，帶 `WWW-Authenticate`）、404、429 |
| `PUT /v1/pastes/{id}/content` | 必須 | 原始文字；加密的 paste 帶 `X-Paste-Password` | 200，新 `ETag` | 401、403、404、409、412、413、429 |
| `GET /v1/pastes/{id}/versions` | — | — | 200 `{"versions": [...]}` | 404 |
| `POST /v1/pastes/{id}/versions/{vid}/content` | — | 同取內容 | 同取內容 | 同取內容 |

- 401 是沒有憑證或憑證無效；403 是憑證有效但這份不屬於你。
- 已過期與不存在一律 404。
- 閱後即焚只能透過 `POST .../content` 且帶 `confirm_burn` 消耗一次，之後一律 404。重試收到 404 時，用戶端要當作「可能已被讀走」處理，不能自動重試。

### 欄位與格式

```json
POST /v1/pastes
{"paste": {
  "content": "...",               必填，UTF-8，位元組數 ≤ WASTEBIN_MAX_BODY_SIZE（線上 64 MiB）
  "extension": "md",              選填，格式規則沿用上游（含 CMakeLists.txt 這類檔名），建立後不能改
  "title": "...",                 選填，≤ 256 字元，null 表示沒有標題
  "expires_in": 604800,           選填，秒；省略或 null 用 WASTEBIN_PASTE_EXPIRATIONS 標記的預設值（線上 7 天）；必須是其中允許的值
  "burn_after_reading": false,
  "password": null                選填；有值就加密
}}

201 Created
Location: /v1/pastes/abc123
ETag: "<version_id>"
{"paste": {
  "id": "abc123",                 6 或 11 字元，與上游相同，不含副檔名
  "extension": "md", "title": "...", "encrypted": false, "burn_after_reading": false,
  "expires_at": "2026-11-21T13:00:00Z",   RFC 3339，UTC
  "version": 1, "version_id": "<64 位小寫十六進位>",
  "owner_token": "...",           只在建立時出現
  "links": [{"rel": "self", "href": ".../v1/pastes/abc123"},
            {"rel": "bookmark", "href": "https://paste.gentoozh.org/abc123.md"}]
}}

GET /v1/pastes?owner=me&limit=20
{"pastes": [...], "pastes_links": [{"rel": "next", "href": ".../v1/pastes?owner=me&limit=20&marker=..."}]}

錯誤（所有端點、包括 axum 自己的拒絕，同一格式）
{"error": {"code": 412, "type": "version_conflict", "title": "Precondition Failed",
           "message": "content changed since version 3", "request_id": "req-..."}}
```

- `GET /v1/pastes/{id}` 只回白名單：不回 uid；加密的 paste 不回標題；`/v1` 的回應都是 `no-store`。
- 列表：先依身分與未過期篩選，再分頁；依 `(created_at, id)` 由新到舊排序，`id` 是同時間的固定次序（新增 `created_at` 欄位，舊 paste 為空，排在最後）。`marker` 是上一頁最後一筆的 id；不屬於這個身分、已刪除或格式錯誤，一律回同一個 400，不透露這個 id 是否存在。`next` 連結保留 `owner=me` 與 `limit`，只提供 `next`。
- `X-Request-Id`：用戶端給的值符合 `[A-Za-z0-9-]{1,64}` 就沿用，否則由伺服器產生；每個回應都帶上。
- 密碼：取內容時放在 JSON body；`PUT` 的 body 是原始文字，所以密碼放 `X-Paste-Password`，值是 UTF-8 的 base64url。
- 所有會跑密碼推導的路徑（取內容、取歷史版本、修改加密 paste、舊路由）共用一個併發上限，並依來源 IP 限速；超過回 429 與 `Retry-After`。
  - 線上 Traefik 已有一層限速（平均 8、突發 40），這裡的限速只針對密碼推導路徑，兩層都保留。
  - 來源 IP：連線來自 Cloudflare 的 IP 範圍（固定清單，隨版本更新）時才採用 `CF-Connecting-IP`，否則用連線本身的位址；其他轉發標頭一律不信任。
  - 源站只接受 Cloudflare 的連線，避免繞過 Cloudflare 直連；matrix 的防火牆按表修改，不用 `flush ruleset`。上游用 Argon2i（記憶體 64 MiB、迭代 10 次），沒有上限時，平行送出錯誤密碼就能耗盡記憶體。
- 日誌遮蔽 `Authorization`、`X-Paste-Password`、`?owner=` 與建立回應的 body。

### 修改與版本

```text
PUT /v1/pastes/{id}/content 與 PATCH /v1/pastes/{id} 走同一套規則
 ├─ 憑證與 Origin（第二節）──不符──▶ 401 或 403
 ├─ 閱後即焚 ──▶ 409
 ├─ 加密的 paste
 │    ├─ 先用提供的密碼解開目前內容 ──失敗──▶ 401（不寫入，避免打錯密碼等於換密碼）
 │    └─ 用同一組密碼與新的 nonce 加密
 └─ 單一交易，依序：
      1 If-Match 帶了且不等於目前版本 ──▶ 412，回傳目前 ETag
      2 內容與標題都和目前版本相同 ──▶ 200，不產生新版本（重送安全）
      3 寫入新版本 ▶ 更新目前指向 ▶ 只保留最近 20 版（含目前這版）
      到期時間不變；回 200 與新的 ETag
```

- 版本 id 用雜湊鏈（同 git commit）：

  ```text
  version_id = sha256( parent_id（32 位元組；第一版為空）
                     ‖ u64be(len(data)) ‖ data          data 是實際儲存的位元組，加密的就是密文
                     ‖ u8(有沒有標題) ‖ u32be(len(title)) ‖ title(UTF-8) )
  輸出 64 位小寫十六進位；ETag 就是 "<version_id>"，照 RFC 9110 強比對
  ```

  - 用密文算，沒有密碼的人不能拿雜湊比對猜內容；內容改回舊值也會因為上一版不同而得到新 id。
  - 每版記錄上一版 id；目前衝突一律 412，歷史是一條直線。是否允許分支日後由 Zephyr 決定。
  - `version` 整數（鏈的深度）只用來排序和顯示「第 N 版」。
- **P1 就建 `revisions` 表**：建立時寫入第一版，所以建立回應的 `version_id` 與 `ETag` 從第一天起就是真的；P2 才開放修改。
- **既有 paste 的補建**：migration 為每一筆舊資料寫入第一版，直接用已儲存的位元組、nonce 與標題計算，不解密，也不會消耗閱後即焚；`extension` 與 `created_at` 留空。線上 3.9.0 升級要先在資料庫副本上驗證。
- `If-Match` 選填，沒帶就直接覆蓋；網頁編輯頁和 `gzpaste edit` 一律帶上與內容在同一次回應中取得的 ETag。
- 歷史存原樣的資料：加密的 paste 存密文與 nonce。paste 刪除或過期時，歷史一併刪除；歷史端點先檢查 paste 是否存在、是否過期，再檢查權限。
- 單一 paste 的歷史總量受 `WASTEBIN_MAX_BODY_SIZE` 的倍數限制。
- 每個分支都有測試：401、403（含 cookie 缺 Origin）、409、412、重送相同內容、交易並發、歷史上限、刪除後歷史消失、429。
- 修改與版本可以整理後送 PR 給上游，但不把上游接受當成維護策略：後端補丁自己維護，並指定一個人追上游的安全更新。
- 之後才做：原生多檔、只限單一 paste 的編輯 token、admin 延長到期、microversion。

### Workspace（專案空間）

用途：同一個任務常有多份計畫、多份結果、審查與日誌。一個 workspace 把它們放在一起，**分享時只給一個連結**，對方打開就看到全部，也能點進任何一份。

```text
paste.gentoozh.org/w/<slug>
 ├─ 寬螢幕：左側清單（依「類別」分組，組內依更新時間），右側是選中那份的閱讀頁
 ├─ 窄螢幕：先看清單，點一份進閱讀頁，可返回
 ├─ 每份仍是一般 paste：有自己的 /<id> 連結、版本歷史、到期與密碼
 └─ /w/<slug>/<名稱> 直接打開 workspace 裡的某一份；分享整個 workspace 就給 /w/<slug>
```

- **資料**：`workspaces` 表（slug、標題、擁有者身分、建立與更新時間）；`entries` 表（workspace、名稱、類別、paste id）。一份 paste 最多屬於一個 workspace；名稱在 workspace 內唯一。
- **slug**：`[a-z0-9-]{2,40}`，先到先得。workspace 裡已經沒有任何未過期的 paste 時，30 天後自動移除，slug 釋出；管理者可以收回被佔用的 slug。
- **類別**：自由文字，例如 `plan`、`result`、`review`、`log`；清單依類別分組，沒填的放在「其他」。同一任務的多份計畫、多份結果就靠這個分開。
- **權限**：知道連結就能看清單（unlisted，不被索引）；加入、移除、改名只有擁有者身分可以。加入一份 paste 時，這份也必須屬於同一個身分。
- **加密的 paste**：清單只顯示名稱與類別，不顯示標題；點進去照常要密碼。
- **刪除 workspace**：只刪分組，不刪裡面的 paste；那些 paste 照自己的到期時間存在，連結仍然有效。

API：

```text
POST   /v1/workspaces                         {"workspace": {"slug", "title"}}；憑證必須
GET    /v1/workspaces/{slug}                  workspace 與清單（名稱、類別、paste id、標題、版本、更新時間）
PATCH  /v1/workspaces/{slug}                  改標題；擁有者
DELETE /v1/workspaces/{slug}                  只刪分組；擁有者
PUT    /v1/workspaces/{slug}/entries/{name}   原始文字；標頭帶 extension、kind
       ├─ 名稱不存在 ──▶ 建立新 paste 並加入，201
       └─ 已存在 ──▶ 對那份 paste 做 PUT content（版本規則同上，可帶 If-Match），200
DELETE /v1/workspaces/{slug}/entries/{name}   移出 workspace，paste 本身不刪；擁有者
```

`PUT .../entries/{name}` 就是 AI 的一次呼叫：第一次建立、之後同名就更新同一份，連結不變。原本只存在本機的「固定名稱」（`gzpaste put -n`）因此取消，改由伺服器端的 workspace 名稱取代，換機器也不會失效。

### 舊路由與契約

- `POST /`、`/raw/<id>` 是薄薄的轉接層，呼叫與 `/v1` 同一組服務函式；同一組情境測試分別打新舊兩條路徑，確認行為一致。
- **契約以 Rust 為準**：用 utoipa 在 handler 上標註，產生 OpenAPI 檔並提交進 repo；前端型別用 openapi-typescript 由這份檔案產生，請求用 openapi-fetch 這個有型別的 client。三個工具都鎖版本，版本在 P0 選定並確認互相相容。
- CI 檢查三件事：
  1. 重新產生 OpenAPI 與前端型別，和 repo 裡的檔案逐位元組比對。
  2. 前端 `tsc` 通過。
  3. 合約測試：在測試程序內起服務，端點表中**本階段已實作**的每一列（含每個錯誤狀態）實際打一次，用 OpenAPI 驗證狀態碼、標頭和 body。只比對產生物，證明不了 handler 真的照契約回應，所以要有這一層。

## 五、gzpaste（openstackclient 風格）

```sh
gzpaste create plan.md            # 簡寫：gzpaste plan.md；stdin 也可以
gzpaste show abc                  # 中繼資料
gzpaste show abc -f json          # -f table|json|value，-c 指定欄位
gzpaste list                      # 伺服器端列出自己的 paste
gzpaste set abc --title "..."     # 改標題
gzpaste edit abc                  # 用 $EDITOR 開啟，存檔即更新，連結不變
gzpaste put abc plan.md           # 覆蓋（給 AI 用）
gzpaste workspace create v2raya --title "v2rayA triage"
gzpaste put -w v2raya -k plan plan-2.md     # 放進 workspace；名稱預設為檔名，第一次建立、之後同名就更新
gzpaste workspace show v2raya               # 清單；-f json 給 AI 讀
gzpaste delete abc
gzpaste version list abc
gzpaste owner                     # 印出帶 ?owner= 的連結，只給自己的其他機器
```

**輸出契約**
- stdout 只放資料；錯誤放 stderr。`-f json` 時錯誤也是 JSON，格式與 API 的錯誤相同。
- JSON 的鍵名與型別就是 API 的欄位名，不翻譯、不隨語系改變；`-c` 只挑欄位，不改名。
- owner token 只有 `gzpaste owner` 會印出，其他輸出一律不含。

**退出碼**

| 碼 | 意義 |
|---|---|
| 0 | 成功 |
| 1 | 其他錯誤 |
| 2 | 用法錯誤 |
| 3 | 不存在或已過期（404） |
| 4 | 驗證或權限（401、403） |
| 5 | 衝突（409、412） |
| 6 | 被限速（429） |
| 7 | 網路錯誤或結果不明（例如閱後即焚的請求逾時） |

**行為**
- 身分：`~/.config/gzpaste/identities`（權限 600）依網站存一份「所有擁有的身分」，其中一個標為目前身分。
  - 建立時帶目前身分；第一次建立沒有身分，就把伺服器回傳的設為目前身分。
  - 匯入其他身分（`gzpaste owner` 印出的連結、舊版存過的 token）只加進清單，不取代目前身分，所以舊的 paste 不會失去管理權。
  - `list` 對清單裡每個身分各查一次再合併；修改與刪除時用擁有那份 paste 的身分。
  - 網頁的 cookie 沿用上游格式（可放多個身分，第一個用來建立），規則相同。
- 寫入身分檔用檔案鎖加暫存檔與 rename，避免兩個 gzpaste 同時第一次建立時互相覆蓋。若伺服器已建立、回應卻在途中遺失，token 就拿不回來；這時以退出碼 7 結束並說明結果不明。
- `edit` 和 `put` 預設帶 `If-Match`；遇到 412 時，有終端機就顯示 diff 問要覆蓋還是放棄，沒有終端機（AI、腳本）就以退出碼 5 結束，不提示也不覆蓋。要強制覆蓋用 `--force`（不帶 `If-Match`）。
- 閱後即焚的讀取不自動重試；逾時以退出碼 7 結束，訊息寫明「可能已被讀走」。
- 參數可以是 id 或任何形式的 paste 網址；憑證只送給設定的網站，不跟隨轉址，也不送給參數裡其他網域的網址。
- 固定名稱改由 workspace 提供（第四節），不再存在本機。

**舊參數對應**

| 舊 | 新 |
|---|---|
| `gzpaste FILE` | `gzpaste create FILE` |
| `-e EXT` | `--extension EXT` |
| `-x SECS` | `--expires-in SECS` |
| `-b` | `--burn` |
| `-p PW` | `--password PW` |
| `-r`、`-m` | `-c raw_url`、`-c bookmark` |
| `-o` | 建立後另外印出 owner 連結（同 `gzpaste owner`） |
| `del <id> <owner>` | `gzpaste delete <id> --owner-token <token>`；沒給 token 就從身分檔取 |

舊參數與新子命令走同一段程式，同一組測試分別以兩種寫法執行。`-c raw_url` 輸出 `/raw/<id>`，`-c bookmark` 輸出 `/<id>.<ext>`。

## 六、階段與驗收

```text
P0 準備
 ├─ 線上設定已盤點（第八節）；.env 為 root 600，上線時以 sudo 讀取
 │    ├─ WASTEBIN_SIGNING_KEY 與 WASTEBIN_PASSWORD_SALT 必須沿用（沒設 salt 時上游預設是 "somesalt"，也要照舊）；
 │    │  否則所有 owner token 失效、加密內容解不開
 │    └─ 線上是 3.9.0（最新 tag），上游 master 已到 c0ec968：先確認 3.9.0 到 master 之間的 migration 與 owner token 格式，
 │       Zpaste 以上游 master 為基礎，上線前在線上資料庫副本驗證
 ├─ 部署：沿用 matrix 上的 docker compose，只把 image 換成自建映像
 │    ├─ GitHub Actions（Zakkaus/zpaste）分三階段建置：Node 建前端 ▶ Rust 建 musl 靜態執行檔並嵌入前端 ▶ 最小映像（distroless 或 scratch），uid 10001 與現在相同
 │    ├─ 推到 ghcr.io/zakkaus/zpaste:<版本>，compose 以 digest 鎖定；回滾就是換回上一個 digest（配合第六節的還原步驟）
 │    └─ 改名範圍：產品名、repo、執行檔、映像、README；內部 crate 名（wastebin_core 等）不改，合併上游修補時衝突較少
 ├─ 桌機開發與測試只用 rootless Docker
 ├─ 改 matrix 防火牆時不用 nft flush ruleset（會清掉 Docker 的 NAT）
 ├─ 先在網站 repo 驗證 Astryx 0.7.0 確實提供 Markdown、Outline、CodeBlock、Table、Tab 與 plugin 介面
 ├─ 主題：新建 gentoo-zh/gentoozh-astryx-theme，只放共用 token 與主題，首頁專用的值留在網站；網站與 Zpaste 都用 git tag 鎖版本
 ├─ 設計語言、架構、工作流三份文件；REQUIREMENTS.md 寫好命令與數字
 ├─ OpenAPI 契約初稿（第四節），送審
 └─ 驗收：改名後、未改功能前，建置出的執行檔對線上資料庫副本的行為與上游相同
P1 /v1 讀取端點 + 閱讀頁
 ├─ /v1 的建立、讀取、刪除、content、列表；revisions 表（建立即寫入第一版）；extension 與 created_at 欄位
 ├─ Rust 外殼（OG、嵌入內容、noscript）；CSP；憑證與 Origin 規則；密碼路徑的併發上限與限速；舊網址轉址
 ├─ 閱讀頁：一個畫面先給 Zakk 看（r8BaRKfkXfi，1280 淺色、390 深色），通過後補齊錯誤、密碼、閱後即焚、三種語系、新建頁
 └─ 驗收：契約重新產生無差異；閱後即焚在按下確認前不被消耗；密碼錯誤回 401 錯誤 JSON；
          Telegram 預覽有標題與摘要；關掉 JS 能讀到純文字；
          390 寬度下表格都是表格；320–1280 無橫向捲動；
          冷載入傳輸量、首次可讀時間、1,600 行文件的捲動都在 REQUIREMENTS.md 的預算內；
          匿名 curl 的 POST / 與 /raw 照常可用；端點表每一列都有合約測試
P1b mermaid 與思維導圖
 ├─ 先做一個畫面給 Zakk 看：markmap 預設樣式與自繪樣式並排，1280 淺色、390 深色
 └─ 驗收：390 寬度是直向可摺疊樹、不需縮放；點節點跳到原文段落；未使用時兩個套件都不被下載
P2 修改與版本
 ├─ PUT content、PATCH、versions、編輯頁、版本頁
 └─ 驗收：第四節每個分支都有測試
P2b Workspace
 ├─ workspaces 與 entries 表、/v1/workspaces 端點、/w/<slug> 頁面（寬螢幕左清單右閱讀、窄螢幕兩層）
 ├─ 先做一個畫面給 Zakk 看：一個有多份計畫與結果的 workspace，1280 淺色、390 深色
 └─ 驗收：AI 用 PUT entries 連續推三次，連結不變、歷史三版；分享 /w/<slug> 一個連結即可看到全部；
          加密的 paste 在清單不露標題；刪 workspace 不刪 paste
P3 gzpaste 子命令（改用 Rust 重寫，放在 Zpaste repo，共用 OpenAPI 型別；Zakk 2026-11-14 同意）
 └─ 驗收：AI 用 put -w 連續推三次，連結不變，歷史留三版；第五節每個退出碼都有測試；
          沒有終端機時遇到 412 不提示也不覆蓋；舊參數與新子命令的測試結果相同
```

上線（P1、P2 各一次）：

```text
目錄（依 FHS 分開設定、軟體與資料；P1 上線時一起搬，舊的 /opt/wastebin 原封保留到穩定為止）
  /opt/zpaste/compose.yml        compose
  /etc/zpaste/env                簽章金鑰與 salt（root 600，從現有設定原值搬過來）
  /var/lib/zpaste/state.db       資料（擁有者 uid 10001）；備份範圍是 /var/lib/zpaste

1 停止舊服務的所有寫入（包括會刪資料的閱後即焚讀取），備份 /opt/wastebin/data/state.db
2 把資料庫複製到 /var/lib/zpaste/（保留 uid 10001）；在另一份副本上跑 migration 與整套測試
3 以 /opt/zpaste/compose.yml 啟動新映像的 digest（沿用簽章金鑰與 salt、Traefik 標籤與限速）
4 抽查：舊 paste、加密 paste、owner 刪除、gzpaste 建立
5 出錯 ──▶ 停 Zpaste ──▶ 直接用原封未動的 /opt/wastebin 啟動舊服務（P2 之後的回滾則還原備份再換回舊 digest）
6 穩定一段時間後才刪 /opt/wastebin
   舊版不能直接開已 migrate 的資料庫（rusqlite_migration 會回 DatabaseTooFarAhead），一定要先還原。
   代價：上線後到回滾之間的新貼文、修改、刪除與已燒掉的閱後即焚都會回到備份當時的狀態。
```

每階段在自己的分支完成，合入後才開始下一階段。核心程式碼由 Opus 撰寫。

不做：帳號系統、配額分級、即時協作、全文搜尋、WYSIWYG。

## 七、風險

| 風險 | 處理 |
|---|---|
| 前端與 API 都換掉，與上游差異大，合併上游困難 | 儲存、加密、到期沿用上游；新 API 是獨立模組；上游前端變更不合入 |
| 使用者內容造成 XSS | 用 Astryx Markdown 與 mermaid strict 的內建防護；只測自己寫的 JSON 嵌入跳脫 |
| mermaid 等依賴撐大 JS | 按內容延遲載入；每個 PR 量測路由體積 |
| 契約與實作分叉 | OpenAPI 由 Rust 產生，前端型別由 OpenAPI 產生；CI 比對產生物，並用合約測試實際打每個端點 |
| 錯誤密碼請求耗盡記憶體 | 所有密碼推導路徑共用併發上限、依 IP 限速、回 429 |
| 共用主題套件讓兩個 repo 綁在一起 | 套件只放共用 token 與主題，發佈成固定版本，不跨 repo 引用原始碼 |
| 換前端期間服務中斷 | 新前端在測試網域驗收後才切換；上線照第六節的步驟 |
| 部署後 owner token 全失效或加密內容解不開 | P0 盤點並沿用 `WASTEBIN_SIGNING_KEY`、`WASTEBIN_PASSWORD_SALT` |

## 八、待 Zakk 決定

| # | 問題 | 建議 |
|---|---|---|
| 1 | repo 與名稱 | **已決定（2026-11-14）**：個人帳號，名稱 Zpaste，非 fork 的獨立 repo，保留上游歷史並註明出處 |
| 2 | 線上 wastebin 的部署 | **已盤點（2026-11-14）**：伺服器 `matrix`（社群 VPS，同機也執行 Matrix 的 Synapse）上的 `/opt/wastebin`，docker compose，映像 `quxfoo/wastebin:3.9.0`，uid 10001，SQLite 在 `./data/state.db`；`WASTEBIN_MAX_BODY_SIZE` 64 MiB；到期選項 `1h,1d,7d=d,1M`（預設 7 天）；`WASTEBIN_SIGNING_KEY` 與 `WASTEBIN_PASSWORD_SALT` 都有設定；前面是 Traefik（網路 `traefik`，限速平均 8、突發 40，壓縮），外層是 Cloudflare |
| 3 | 共用主題套件放哪 | **已決定：獨立 repo**（`gentoo-zh/gentoozh-astryx-theme`）。兩個使用者在不同帳號下，各自用 git tag 鎖版本；網站 repo 不用多管發佈流程 |
| 4 | 驗證標頭 | **已決定：只用 `Authorization: Bearer`** |

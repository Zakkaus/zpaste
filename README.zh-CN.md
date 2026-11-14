# Zpaste

[English](README.md) | [繁體中文](README.zh-TW.md) | 简体中文

Zpaste 是自托管的 pastebin，用于阅读 Markdown、代码和日志，无论内容由人编写还是由 AI 工具生成。它将取代 <https://paste.gentoozh.org> 上的 wastebin，让该站的长篇内容便于阅读。在同一 URL 修改 paste 和保留版本历史已列入计划。

Zpaste 基于 Matthias Vogelgesang 的 [wastebin](https://github.com/matze/wastebin)，沿用 wastebin 的存储、加密、过期和阅后即焚代码。

## 状态

Zpaste 仍处于早期开发阶段。目前的行为与上游 wastebin 提交 `c0ec968` 相同，区别只有服务器可执行文件名为 `zpaste`，以及日志行的 target 显示为 `zpaste`。配置变量仍使用 `WASTEBIN_` 前缀，HTTP API 也与 wastebin 相同。

## 构建

构建需要 Rust 1.95 或更高版本和 C 编译器，因为 SQLite 和 zstd 从源码编译。在仓库根目录执行：

```sh
cargo build --release
```

服务器输出到 `target/release/zpaste`，数据库工具输出到 `target/release/wastebin-ctl`。

## 运行

使用当前目录下的数据库文件启动服务器：

```sh
WASTEBIN_DATABASE_PATH=state.db ./target/release/zpaste
```

服务器监听 `0.0.0.0:8088`。未设置 `WASTEBIN_DATABASE_PATH` 时，paste 只保存在内存中，程序退出即丢失。用浏览器打开 <http://127.0.0.1:8088>，或在 shell 中创建 paste：

```sh
curl -s -H 'Content-Type: application/json' \
    --data '{"text": "# Hello", "extension": "md"}' \
    http://127.0.0.1:8088/
```

响应包含新 paste 的路径，以及可以删除它的 owner token：

```json
{"path":"/<id>.md","owner":"<token>"}
```

`curl http://127.0.0.1:8088/raw/<id>` 返回保存的文本。配置变量、其余 HTTP API 和 `wastebin-ctl` 的说明见 [wastebin 的 README](docs/upstream/WASTEBIN-README.md#configuration)。

## 容器镜像

Dockerfile 交叉编译静态链接 musl 的可执行文件，设计为在 x86_64 主机上使用。在仓库根目录构建 x86_64 镜像：

```sh
docker build --target amd64 -t zpaste .
```

镜像以用户 10001 运行 `/app/zpaste`。

## 测试

```sh
cargo test
```

## 计划

- 阅读页：渲染 Markdown 并附目录，代码、日志和 diff 各有合适的视图。
- 在同一 URL 修改 paste，并保留版本历史。
- 位于 `/v1` 下、带版本号的 HTTP API，以及命令行客户端。

工作计划见 [docs/plan.zh-TW.md](docs/plan.zh-TW.md)（繁体中文）。

## 上游文档

wastebin 的 README、变更日志和贡献指南保留在 [docs/upstream](docs/upstream/README.md) 供参考。这些文档描述的是 wastebin，会随 Zpaste 的修改而与现状不符。

## 许可证

Zpaste 以 [MIT 许可证](LICENSE)发布。许可证文件保留 Matthias Vogelgesang 对 wastebin 的版权声明，并加上 Zakkaus 对 Zpaste 的版权声明。

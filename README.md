# rserve

**rserve** is a fast and minimal static file server and directory indexer written in Rust. It serves any folder over HTTP with zero configuration — perfect for local development, quick file sharing, or testing static sites.

---

## 🚀 Features

- 🧱 Serve any local directory over HTTP
- 📁 Auto-generated directory listings
- ⚡️ Blazing fast and minimal (Rust-powered)
- 🛠 Simple CLI with customizable host and port

---

## 📦 Build & Install

```bash
git clone https://github.com/gustavosvalentim/rserve.git
cd rserve
cargo build --release
```

The compiled binary will be located at `target/release/rserve`.

---

## ▶️ Usage

```bash
rserve
```

This will serve the current directory at `http://localhost:8080`.

---

## 📌 CLI Options

| Option         | Description                   | Default   |
| -------------- | ----------------------------- | --------- |
| `-H`, `--host` | Set custom host or IP address | `0.0.0.0` |
| `-P`, `--port` | Set custom port               | `8000`    |

---

## 🧪 Examples

Serve a folder on a custom port:

```bash
cd ~/my-folder
rserve -P 9000
```

Bind to a specific host:

```bash
rserve -H 127.0.0.1
```

Serve a specific folder directly:

```bash
cd /path/to/static/site
rserve
```

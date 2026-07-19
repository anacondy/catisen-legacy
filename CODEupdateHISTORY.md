# 📜 CODEupdateHISTORY

This file tracks all code updates, commits, and changes made to the **Catisen** repository. It’s a simple table format to keep track of **who added what, when, and how much**.

---

## 📌 How to Use This File
- **Each entry** represents a new feature, bug fix, or update.
- **Columns**: Date, Commit Hash, Feature Added, Lines Added, Lines Deleted, Git Version.
- **Link to this file** is present in the [README.md](README.md) for easy access.

---

## 📅 Update Log

| **Date**       | **Commit Hash**       | **Feature Added**               | **Lines Added** | **Lines Deleted** | **Git Version** |
|----------------|-----------------------|----------------------------------|-----------------|-------------------|-----------------|
| 2026-03-29     | `43602dd`             | Rust/Servo starter code, GTK4 theme | 120             | 0                 | v1.0            |
| 2026-03-29     | `new_commit_hash`     | uBlock Origin integration        | 45              | 0                 | v1.1            |
| 2026-03-29     | `new_commit_hash`     | libcurl download manager         | 60              | 0                 | v1.2            |
| 2026-03-29     | `new_commit_hash`     | FFmpeg media player              | 75              | 0                 | v1.3            |

---

## 🔗 Important Links
- **[README.md](README.md)** (Top link: "📜 CODEupdateHISTORY")
- **[GitHub Repo](https://github.com/anacondy/catisen)**

---

## 💡 How to Continue
1. **Pull the latest changes**:
   ```bash
   git pull origin main
   ```
2. **Test the new features** in VS Code.
3. **Document your changes** in this file before pushing.
4. **Push with a clear message** (e.g., "Add uBlock Origin integration").## April 2026 Core Updates
- **Persistent Configuration (\CatisenConfig\)**: App settings save and load cleanly via a unified config.toml via serde.
- **Invisible Headless Test Mode**: Integration tests via CATISEN_TEST_MODE=1 invoke an invisible eframe viewport to natively route eqwest clients synchronously with the SOCKS5 proxies without duplicated logic or flashing UI frames.
- **Robust SOCKS5 Tor Routing**: eqwest features upgraded seamlessly; .onion urls correctly bounce against 127.0.0.1:9150 via 	or_manager logging.
## April 2026 Core Updates
- **Persistent Configuration (\CatisenConfig\)**: App settings save and load cleanly via a unified config.toml via serde.
- **Invisible Headless Test Mode**: Integration tests via CATISEN_TEST_MODE=1 invoke an invisible eframe viewport to natively route eqwest clients synchronously with the SOCKS5 proxies without duplicated logic or flashing UI frames.
- **Robust SOCKS5 Tor Routing**: eqwest features upgraded seamlessly; .onion urls correctly bounce against 127.0.0.1:9150 via 	or_manager logging.

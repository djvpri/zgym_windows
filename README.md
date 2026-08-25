# ZGym Desktop

Aplikasi Windows (thin-client Tauri v2) untuk **ZGym** — SaaS gym management.
App ini hanya *shell* yang membuka ZGym prod di WebView2. Semua logika,
auth & data tetap di server (Postgres Railway). Tidak ada kode ulang backend.

## Cara kerja

- Window Tauri v2 membuka `https://zgym-production.up.railway.app` (default).
- Override URL staging/uji: set env `ZGym_URL` saat build/run.
- Login & session = cookie `authjs.session-token` normal di webview (persist
  antar-restart). Tiap staf gym login ke akun mereka (multi-tenant via server).
- Single-instance: jalankan lagi = fokus ulang window yang ada.

## Build (Windows)

CI `build-windows.yml` compile + paket NSIS di `windows-latest`. Hasil:
`ZGym_<versi>_x64-setup.exe` (install ke current user, satu folder).

```bash
cd src-tauri
cargo tauri build
```

## Struktur

```
src-tauri/
  Cargo.toml        # deps Tauri v2 + single-instance
  src/lib.rs        # window WebviewUrl::External -> ZGym prod, env ZGym_URL
  src/main.rs       # entrypoint (windows_subsystem)
  tauri.conf.json   # productName, identifier com.zomet.zgym.desktop, NSIS
dist/index.html     # frontendDist kosong (window selalu load URL eksternal)
```

> Toolchain: Rust stable + cargo-tauri CLI v2. Butuh WebView2 (bawaan Win 10/11).

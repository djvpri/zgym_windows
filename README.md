# ZXgym Desktop

Aplikasi Windows (thin-client Tauri v2) untuk **ZXgym** — SaaS gym management.
App ini hanya *shell* yang membuka ZXgym prod di WebView2. Semua logika,
auth & data tetap di server. Tidak ada kode ulang backend.

## Cara kerja

- Window Tauri v2 membuka `https://zxgym.zomet.my.id/login` (default).
- Override URL staging/uji: set env `ZXgym_URL` saat build/run.
- Login & session = cookie `authjs.session-token` normal di webview (persist
  antar-restart). Tiap staf gym login ke akun mereka (multi-tenant via server).
- Single-instance: jalankan lagi = fokus ulang window yang ada.

## Build (Windows)

CI `build-windows.yml` compile + paket NSIS di `windows-latest`. Hasil:
`ZXgym_<versi>_x64-setup.exe` (install ke current user, satu folder).

```bash
cd src-tauri
cargo tauri build
```

## Struktur

```
src-tauri/
  Cargo.toml        # deps Tauri v2 + single-instance
  src/lib.rs        # window WebviewUrl::External -> ZXgym prod, env ZXgym_URL
  src/main.rs       # entrypoint (windows_subsystem)
  tauri.conf.json   # productName, identifier com.zomet.zxgym.desktop, NSIS
dist/index.html     # frontendDist kosong (window selalu load URL eksternal)
```

> Toolchain: Rust stable + cargo-tauri CLI v2. Butuh WebView2 (bawaan Win 10/11).

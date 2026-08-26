// ZXgym Desktop — thin-client Tauri v2.
// Hanya {shell webview} yang membuka ZXgym SaaS ke URL prod (atau override env
// ZXgym_URL). Semua logika, auth, data tetap server-side (Postgres di VPS);
// app ini nol kode ulang backend. Konsisten dgn arsitektur multi-tenant ZXgym:
// tiap staf gym login ke akun mereka, session cookie persist di webview.
use tauri::{
    Manager, WebviewUrl, WebviewWindowBuilder,
};

// URL default ZXgym prod (langsung ke login). Override via env ZXgym_URL
// (staging/uji lokal). Kalau session uda aktif, NextAuth redirect ke dashboard.
fn zxgym_url() -> String {
    std::env::var("ZXgym_URL")
        .unwrap_or_else(|_| "https://zxgym.zomet.my.id/login".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let url = zxgym_url();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {
            // Instance kedua: fokus ulang window utama.
            if let Some(w) = _app.get_webview_window("main") {
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |app| {
            // `move` menangkap `url` by value (closure `'static`; hindari
            // E0373: closure outlive pemilik `url`).
            // Buka ZXgym di window utama. `WebviewUrl::External` = load URL
            // tambahan langsung, bukan asset lokal index.html.
            let _w = WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::External(url.parse::<tauri::Url>().unwrap()),
            )
            .title("ZXgym")
            .inner_size(1280.0, 820.0)
            .min_inner_size(960.0, 640.0)
            .build()
            .expect("Failed to build ZXgym window");

            // Auto-update: cek latest.json tiap startup. Ada versi baru ->
            // download + install (NSIS). Gagal diam-diam (app tetap jalan).
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                use tauri_plugin_updater::UpdaterExt;
                let Ok(updater) = handle.updater() else { return };
                match updater.check().await {
                    Ok(Some(u)) => {
                        let _ = u
                            .download_and_install(|_chunk, _len| {}, || {})
                            .await;
                    }
                    _ => {}
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running ZXgym desktop");
}

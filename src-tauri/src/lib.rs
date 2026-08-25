// ZGym Desktop — thin-client Tauri v2.
// Hanya {shell webview} yang membuka ZGym SaaS ke URL prod (atau override env
// ZGym_URL). Semua logika, auth, data tetap server-side (Postgres Railway);
// app ini nol kode ulang backend. Konsisten dgn arsitektur multi-tenant ZGym:
// tiap staf gym login ke akun mereka, session cookie persist di webview.
use tauri::{
    Manager, WebviewUrl, WebviewWindowBuilder,
};

// URL default ZGym prod (langsung ke login). Override via env ZGym_URL
// (staging/uji lokal). Kalau session uda aktif, NextAuth redirect ke dashboard.
fn zgym_url() -> String {
    std::env::var("ZGym_URL")
        .unwrap_or_else(|_| "https://zgym-production.up.railway.app/login".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let url = zgym_url();

    tauri::Builder::default()
        .plugin(
            tauri_plugin_single_instance::init(|_app, _args, _cwd| {
                // Instance kedua: fokus ulang window utama.
                if let Some(w) = _app.get_webview_window("main") {
                    let _ = w.set_focus();
                }
            }),
        )
        .setup(move |app| {
            // `move` menangkap `url` by value (closure `'static`; hindari
            // E0373: closure outlive pemilik `url`).
            // Buka ZGym di window utama. `WebviewUrl::External` = load URL
            // tambahan langsung, bukan asset lokal index.html.
            let _w = WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::External(url.parse::<tauri::Url>().unwrap()),
            )
            .title("ZGym")
                .inner_size(1280.0, 820.0)
                .min_inner_size(960.0, 640.0)
                .build()
                .expect("Failed to build ZGym window");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running ZGym desktop");
}

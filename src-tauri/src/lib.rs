// ZXgym Desktop — thin-client Tauri v2.
// Hanya {shell webview} yang membuka ZXgym SaaS ke URL prod (atau override env
// ZXgym_URL). Semua logika, auth, data tetap server-side (Postgres di VPS);
// app ini nol kode ulang backend. Konsisten dgn arsitektur multi-tenant ZXgym:
// tiap staf gym login ke akun mereka, session cookie persist di webview.
use tauri::{
    Manager, WebviewUrl, WebviewWindowBuilder,
};

// ===== Cetak nota thermal (ESC/POS) via Windows Print Spooler (winspool) =====
// Disalin dari pola z1-kasir (zpos-kasir) yang terbukti hasil nota paling bagus:
// terjemahkan nota ke byte ESC/POS mentah di frontend, kirim LANGSUNG ke driver
// printer via OpenPrinterW/StartDocPrinterW/WritePrinter (pDatatype "RAW").
// Printer render dgn DPI aslinya (bukan lewat dialog browser) -> tegas, tak buram.

/// Daftar printer terpasang di Windows. Dipakai frontend utk dropdown "Printer".
/// Salin pola z1-kasir: PRINTER_INFO_1W + flags local|connections (biar printer
/// USB lokal & shared jaringan ikut; BT virtual COM di luar spooler — tak muncul).
#[tauri::command]
fn daftar_printer() -> Result<Vec<String>, String> {
    #[cfg(windows)]
    {
        use windows::Win32::Graphics::Printing::{EnumPrintersW, PRINTER_INFO_1W};
        const PRINTER_ENUM_LOCAL: u32 = 0x00000002;
        const PRINTER_ENUM_CONNECTIONS: u32 = 0x00000004;
        let flags: u32 = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;
        let mut needed: u32 = 0;
        let mut returned: u32 = 0;
        // pass 1: hitung ukuran buffer. EnumPrintersW dgn buffer None (NULL) return
        // FALSE + ERROR_INSUFFICIENT_BUFFER (0x8007007A) utk query ukuran — ini
        // perilaku DUA-PASS yg diharapkan, BUKAN gagal. Jangan `?`; baca `needed`.
        let _hr = unsafe { EnumPrintersW(flags, None, 1, None, &mut needed, &mut returned) };
        if needed == 0 {
            return Ok(Vec::new());
        }
        let mut buf = vec![0u8; needed as usize];
        let mut returned2: u32 = 0;
        unsafe {
            EnumPrintersW(
                flags,
                None,
                1,
                Some(&mut buf[..]),
                &mut needed,
                &mut returned2,
            )
        }
        .map_err(|e| format!("EnumPrintersW (isi) gagal: {e}"))?;
        let stride = std::mem::size_of::<PRINTER_INFO_1W>();
        let n = returned2 as usize;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let base = buf.as_ptr() as usize + i * stride;
            let info = unsafe { &*(base as *const PRINTER_INFO_1W) };
            unsafe {
                if !info.pName.is_null() {
                    if let Ok(name) = info.pName.to_string() {
                        out.push(name);
                    }
                }
            }
        }
        Ok(out)
    }
    #[cfg(not(windows))]
    {
        Err("Cetak via spooler hanya didukung di Windows (ZXgym).".into())
    }
}

/// Kirim byte ESC/POS mentah ke printer thermal terpilih.
#[tauri::command]
fn cetak_escpos(escpos: String, nama_printer: String) -> Result<String, String> {
    #[cfg(windows)]
    {
        use std::os::raw::c_void;
        use windows::core::{w, PCWSTR, PWSTR};
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::Graphics::Printing::{
            ClosePrinter, EndDocPrinter, OpenPrinterW, StartDocPrinterW, WritePrinter, DOC_INFO_1W,
        };
        if nama_printer.trim().is_empty() {
            return Err("Nama printer kosong.".into());
        }
        let name16: Vec<u16> = nama_printer.encode_utf16().chain(Some(0)).collect();
        let pname = PCWSTR(name16.as_ptr());
        let mut hprinter: HANDLE = HANDLE::default();
        unsafe { OpenPrinterW(pname, &mut hprinter, None) }.map_err(|_| {
            format!(
                "Printer \"{nama_printer}\" tidak bisa dibuka. Pastikan driver thermal terpasang di Printers & scanners.",
            )
        })?;
        let r = (|| -> Result<String, String> {
            let doc = DOC_INFO_1W {
                pDocName: PWSTR(w!("ZXgym nota").as_ptr() as *mut u16),
                pOutputFile: PWSTR::null(),
                pDatatype: PWSTR(w!("RAW").as_ptr() as *mut u16),
            };
            let job: u32 = unsafe { StartDocPrinterW(hprinter, 1, &doc) };
            if job == 0 {
                return Err("StartDocPrinter gagal.".into());
            }
            let data = escpos.as_bytes();
            let mut written: u32 = 0;
            let okw: windows::Win32::Foundation::BOOL = unsafe {
                WritePrinter(hprinter, data.as_ptr() as *const c_void, data.len() as u32, &mut written)
            };
            let _ = unsafe { EndDocPrinter(hprinter) };
            if !okw.as_bool() {
                return Err("WritePrinter gagal mengirim ESC/POS.".into());
            }
            Ok(format!("Terkirim {} byte ke {}.", written, nama_printer))
        })();
        let _ = unsafe { EndDocPrinter(hprinter) };
        let _ = unsafe { ClosePrinter(hprinter) };
        r
    }
    #[cfg(not(windows))]
    {
        Err("Cetak ESC/POS hanya didukung di Windows desktop (ZXgym).".into())
    }
}

// URL default ZXgym prod (langsung ke dashboard). Override via env ZXgym_URL
// (staging/uji lokal). Middleware NextAuth otomatis: kalau session ada -> dashboard;
// kalau belum -> redirect /login -> Z One (hub SSO) -> balik /sso -> dashboard.
fn zxgym_url() -> String {
    std::env::var("ZXgym_URL")
        .unwrap_or_else(|_| "https://zxgym.zomet.my.id/dashboard".to_string())
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
        .invoke_handler(tauri::generate_handler![daftar_printer, cetak_escpos])
        .setup(move |app| {
            // `move` menangkap `url` by value (closure `'static`; hindari
            // E0373: closure outlive pemilik `url`).
            // Buka ZXgym di window utama. `WebviewUrl::External` = load URL
            // tambahan langsung, bukan asset lokal index.html.
            // Persist WebView2 user-data ke folder data app supaya cookie session
            // (NextAuth/SSO) TIDAK hilang antar restart -> user tak perlu login ulang.
            let wv_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to resolve app data dir");
            let _w = WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::External(url.parse::<tauri::Url>().unwrap()),
            )
            .title("ZXgym")
            .inner_size(1280.0, 820.0)
            .min_inner_size(960.0, 640.0)
            .data_directory(wv_data_dir)
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

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Desactiva el renderizador DMA-BUF de WebKitGTK.
    //
    // En algunos sistemas Linux con Wayland puede provocar:
    // "Gdk-Message: Error 71 (Error de protocolo)"
    //
    // RepoFy funciona correctamente con este renderizador desactivado.
    #[cfg(target_os = "linux")]
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

    repofy_lib::run()
}
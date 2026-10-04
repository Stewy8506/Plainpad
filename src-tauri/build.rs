fn main() {
    // Ensure ../ui/dist exists so tauri::generate_context!() never panics on a fresh clone
    let dist = std::path::Path::new("../ui/dist");
    if !dist.exists() {
        let _ = std::fs::create_dir_all(dist);
        let _ = std::fs::write(
            dist.join("index.html"),
            "<!DOCTYPE html><html><head><title>Plainpad</title></head><body>Loading Plainpad...</body></html>",
        );
    }
    tauri_build::build()
}

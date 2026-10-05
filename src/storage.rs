//! What survives between runs: the coin bank (and, soon, permanent upgrades).
//! The browser build keeps it in localStorage through a tiny JS plugin in
//! web/template.html; desktop builds keep a small text file.

#[derive(Clone, Copy, Default, Debug)]
pub struct Meta {
    pub bank: u32,
}

impl Meta {
    pub fn load() -> Meta {
        let mut m = Meta::default();
        for line in read().lines() {
            if let Some((k, v)) = line.split_once('=') {
                if k.trim() == "bank" {
                    m.bank = v.trim().parse().unwrap_or(0);
                }
            }
        }
        m
    }

    pub fn save(&self) {
        write(&format!("bank={}\n", self.bank));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn path() -> Option<std::path::PathBuf> {
    let base = std::env::var("APPDATA").or_else(|_| std::env::var("HOME")).ok()?;
    Some(std::path::PathBuf::from(base).join(".last-train-to-shinjuku").join("save.txt"))
}

#[cfg(not(target_arch = "wasm32"))]
fn read() -> String {
    path().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
fn write(data: &str) {
    if let Some(p) = path() {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(p, data);
    }
}

#[cfg(target_arch = "wasm32")]
unsafe extern "C" {
    fn sj_store_get(ptr: *mut u8, cap: u32) -> u32;
    fn sj_store_set(ptr: *const u8, len: u32);
}

#[cfg(target_arch = "wasm32")]
fn read() -> String {
    let mut buf = vec![0u8; 4096];
    let n = unsafe { sj_store_get(buf.as_mut_ptr(), buf.len() as u32) } as usize;
    buf.truncate(n.min(4096));
    String::from_utf8(buf).unwrap_or_default()
}

#[cfg(target_arch = "wasm32")]
fn write(data: &str) {
    unsafe { sj_store_set(data.as_ptr(), data.len() as u32) };
}

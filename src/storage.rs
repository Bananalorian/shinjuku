//! What survives between runs: the coin bank, the permanent upgrades bought with
//! it, and whether you've seen the opening. The browser build keeps it in
//! localStorage through a tiny JS plugin in web/template.html; desktop builds keep
//! a small text file.

use crate::shop::PERKS;

#[derive(Clone, Copy, Debug)]
pub struct Meta {
    pub bank: u32,
    pub levels: [u8; PERKS.len()],
    pub seen_intro: bool,
    pub skip_intro: bool,
}

impl Default for Meta {
    fn default() -> Self {
        Meta { bank: 0, levels: [0; PERKS.len()], seen_intro: false, skip_intro: false }
    }
}

impl Meta {
    pub fn load() -> Meta {
        let mut m = Meta::default();
        for line in read().lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            let (k, v) = (k.trim(), v.trim());
            match k {
                "bank" => m.bank = v.parse().unwrap_or(0),
                "seen_intro" => m.seen_intro = v == "1",
                "skip_intro" => m.skip_intro = v == "1",
                _ => {
                    if let Some(i) = PERKS.iter().position(|p| p.key == k) {
                        m.levels[i] = v.parse::<u8>().unwrap_or(0).min(PERKS[i].costs.len() as u8);
                    }
                }
            }
        }
        m
    }

    pub fn save(&self) {
        let mut s = format!("bank={}\nseen_intro={}\nskip_intro={}\n", self.bank, self.seen_intro as u8, self.skip_intro as u8);
        for (i, p) in PERKS.iter().enumerate() {
            s.push_str(&format!("{}={}\n", p.key, self.levels[i]));
        }
        write(&s);
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

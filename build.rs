// Embed the multi-size ICO as standard Windows RT_ICON / RT_GROUP_ICON
// resources. Generates MSVC .res directly; no SDK resource compiler required.
use std::{env, fs, path::PathBuf};

fn word(out: &mut Vec<u8>, n: u16) {
    out.extend(n.to_le_bytes());
}
fn dword(out: &mut Vec<u8>, n: u32) {
    out.extend(n.to_le_bytes());
}
fn get_word(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes(b[i..i + 2].try_into().unwrap())
}
fn get_dword(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes(b[i..i + 4].try_into().unwrap())
}
fn record(out: &mut Vec<u8>, kind: u16, id: u16, data: &[u8]) {
    dword(out, data.len().try_into().unwrap());
    dword(out, 32);
    word(out, 0xFFFF);
    word(out, kind);
    word(out, 0xFFFF);
    word(out, id);
    dword(out, 0);
    word(out, 0x1030);
    word(out, 0);
    dword(out, 0);
    dword(out, 0);
    out.extend(data);
    while !out.len().is_multiple_of(4) {
        out.push(0);
    }
}
fn main() {
    println!("cargo:rerun-if-changed=assets/barker.ico");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    assert_eq!(
        env::var("CARGO_CFG_TARGET_ENV").unwrap(),
        "msvc",
        "Icon embedding requires the Windows MSVC target"
    );
    let ico = fs::read("assets/barker.ico").expect("Missing assets/barker.ico");
    assert!(
        ico.len() >= 6 && get_word(&ico, 0) == 0 && get_word(&ico, 2) == 1,
        "Invalid ICO"
    );
    let count = get_word(&ico, 4);
    assert!(count > 0 && ico.len() >= 6 + usize::from(count) * 16);
    let mut res = Vec::new();
    record(&mut res, 0, 0, &[]);
    // Null resource header has no memory flags.
    res[20..22].copy_from_slice(&0u16.to_le_bytes());
    let mut group = ico[..6].to_vec();
    for i in 0..count {
        let pos = 6 + usize::from(i) * 16;
        let size = get_dword(&ico, pos + 8) as usize;
        let offset = get_dword(&ico, pos + 12) as usize;
        let payload = ico
            .get(offset..offset.checked_add(size).unwrap())
            .expect("Truncated ICO");
        record(&mut res, 3, i + 1, payload);
        group.extend(&ico[pos..pos + 12]);
        word(&mut group, i + 1);
    }
    record(&mut res, 14, 101, &group);
    let path = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("barker.res");
    fs::write(&path, res).unwrap();
    println!("cargo:rustc-link-arg-bins={}", path.display());
}

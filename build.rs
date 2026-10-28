use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=assets/icons/icon.ico");

    if env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() != "windows" {
        return;
    }

    let ico_path = PathBuf::from("assets/icons/icon.ico");
    let Ok(ico) = fs::read(&ico_path) else {
        return;
    };
    if ico.len() < 22 {
        return;
    }

    let count = u16::from_le_bytes([ico[4], ico[5]]) as usize;
    if count == 0 || ico.len() < 6 + count * 16 {
        return;
    }

    let mut res = Vec::new();

    // 1. Mandatory 32-byte empty resource header
    write_res_header(&mut res, 0, 0, 0, 0, 0);

    // 2. Write each RT_ICON (Type = 3) entry with ID = (i + 1)
    let mut grp_dir = Vec::new();
    grp_dir.extend_from_slice(&[0, 0, 1, 0]);
    grp_dir.extend_from_slice(&(count as u16).to_le_bytes());

    for i in 0..count {
        let entry_off = 6 + i * 16;
        let size = u32::from_le_bytes([
            ico[entry_off + 8],
            ico[entry_off + 9],
            ico[entry_off + 10],
            ico[entry_off + 11],
        ]) as usize;
        let data_off = u32::from_le_bytes([
            ico[entry_off + 12],
            ico[entry_off + 13],
            ico[entry_off + 14],
            ico[entry_off + 15],
        ]) as usize;

        if data_off + size > ico.len() {
            continue;
        }

        let icon_id = (i + 1) as u16;
        // RT_ICON = 3
        write_res_header(&mut res, size as u32, 3, icon_id, 0x1010, 0x0409);
        res.extend_from_slice(&ico[data_off..data_off + size]);
        align4(&mut res);

        // Append 14-byte GRPICONDIRENTRY (first 12 bytes of ICONDIRENTRY + 2-byte icon_id)
        grp_dir.extend_from_slice(&ico[entry_off..entry_off + 12]);
        grp_dir.extend_from_slice(&icon_id.to_le_bytes());
    }

    // 3. Write RT_GROUP_ICON (Type = 14) with ID = 1
    write_res_header(&mut res, grp_dir.len() as u32, 14, 1, 0x1030, 0x0409);
    res.extend_from_slice(&grp_dir);
    align4(&mut res);

    if let Ok(out_dir) = env::var("OUT_DIR") {
        let res_path = PathBuf::from(out_dir).join("nexora_icon.res");
        if fs::write(&res_path, &res).is_ok() {
            println!("cargo:rustc-link-arg-bins={}", res_path.display());
        }
    }
}

fn write_res_header(
    out: &mut Vec<u8>,
    data_size: u32,
    type_id: u16,
    name_id: u16,
    mem_flags: u16,
    lang_id: u16,
) {
    out.extend_from_slice(&data_size.to_le_bytes()); // DataSize
    out.extend_from_slice(&32u32.to_le_bytes());     // HeaderSize = 32
    out.extend_from_slice(&0xFFFFu16.to_le_bytes()); // Type ordinal marker
    out.extend_from_slice(&type_id.to_le_bytes());   // Type ID
    out.extend_from_slice(&0xFFFFu16.to_le_bytes()); // Name ordinal marker
    out.extend_from_slice(&name_id.to_le_bytes());   // Name ID
    out.extend_from_slice(&0u32.to_le_bytes());      // DataVersion
    out.extend_from_slice(&mem_flags.to_le_bytes()); // MemoryFlags
    out.extend_from_slice(&lang_id.to_le_bytes());   // LanguageId
    out.extend_from_slice(&0u32.to_le_bytes());      // Version
    out.extend_from_slice(&0u32.to_le_bytes());      // Characteristics
}

fn align4(out: &mut Vec<u8>) {
    while out.len() % 4 != 0 {
        out.push(0);
    }
}


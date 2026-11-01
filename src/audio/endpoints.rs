//! Shared Windows audio endpoint helpers.
//!
//! Both the Audio Mixer's live meters ([`super::wasapi_audio`]) and the recording encoder need
//! to answer the same two questions:
//!
//! 1. *"Which active endpoint did the user mean by this friendly name?"*
//! 2. *"Which PID is this executable?"*
//!
//! Answering them in one place keeps the two subsystems in agreement — in particular it keeps
//! the device list built from WinMM / Media Foundation reconcilable with `IMMDevice`'s
//! `PKEY_Device_FriendlyName`, which is spelled differently for the same physical device.

use std::mem::size_of;

use windows::core::PWSTR;
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::StructuredStorage::{
    PropVariantClear, PropVariantToBSTR, PROPVARIANT,
};
use windows::Win32::System::Com::*;
use windows::Win32::System::Diagnostics::ToolHelp::*;

/// Folds a Windows endpoint friendly name into a comparison-friendly form.
///
/// Removes everything that commonly differs between the WinMM, Media Foundation and MMDevice
/// spellings of the *same* physical device (spaces, dashes, brackets, vendor punctuation) so
/// `"Microphone Array (2- Intel® Smart Sound Technology for Di…"` still matches
/// `"Microphone Array (Intel® Smart Sound Technology for Digital Microphones)"`.
pub fn normalize_device_name(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Splits a device name into lowercase alphanumeric tokens.
///
/// Bare numbers are dropped because WinMM appends duplicate indices, producing names such as
/// `"Microphone Array (2- Realtek(R) Audio)"` for a device MMDevice calls plain
/// `"Microphone Array (Realtek(R) Audio)"`.
fn device_tokens(raw: &str) -> Vec<String> {
    raw.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_lowercase())
        .filter(|t| !t.chars().all(|c| c.is_ascii_digit()))
        .collect()
}

/// Fraction of the `want` tokens that appear in `candidate`.
///
/// A candidate token counts as a match when it equals the wanted token or starts with it, which
/// absorbs the UI's mid-word truncation (`"di…"` matching `"digital"`). This is what makes a
/// truncated combo-box label still resolve to the right physical device.
fn token_coverage(want: &str, candidate: &str) -> f32 {
    let w = device_tokens(want);
    let c = device_tokens(candidate);
    if w.is_empty() || c.is_empty() {
        return 0.0;
    }
    let matched = w
        .iter()
        .filter(|wt| {
            let wt_len = wt.chars().count();
            c.iter()
                .any(|ct| ct == *wt || (wt_len >= 3 && ct.starts_with(wt.as_str())))
        })
        .count();
    matched as f32 / w.len() as f32
}

/// Scores how well `candidate` satisfies the requested `want` name.
///
/// Higher is better; `0` means "not a match at all". Used to pick the best endpoint from
/// `EnumAudioEndpoints` without requiring an exact (and therefore brittle) string equality.
pub fn score_device_name(want: &str, candidate: &str) -> i32 {
    let want_trim = want.trim();
    let cand_trim = candidate.trim();
    if want_trim.is_empty() || cand_trim.is_empty() {
        return 0;
    }
    if want_trim.eq_ignore_ascii_case(cand_trim) {
        return 1_000;
    }

    let wn = normalize_device_name(want_trim);
    let cn = normalize_device_name(cand_trim);
    if wn.is_empty() || cn.is_empty() {
        return 0;
    }
    if wn == cn {
        return 900;
    }

    // One side truncated by the UI ("... for Di…"): accept a prefix relationship.
    let shorter = wn.len().min(cn.len());
    if shorter >= 8 {
        if wn.starts_with(&cn) || cn.starts_with(&wn) {
            return 700 + shorter as i32;
        }
        if wn.contains(&cn) || cn.contains(&wn) {
            return 500 + shorter as i32;
        }
    }

    // Leading token heuristic: the part before the first "(" is usually the device class
    // ("Speakers", "Microphone Array", "Headphones") and is highly discriminative.
    let wt = wn.split(|c: char| !c.is_ascii_alphanumeric()).next().unwrap_or("");
    let ct = cn.split(|c: char| !c.is_ascii_alphanumeric()).next().unwrap_or("");
    if !wt.is_empty() && wt == ct && wt.len() >= 4 {
        return 300;
    }

    // Token-containment fallback: tolerates truncation, vendor punctuation and WinMM's
    // duplicate-index digits. Requires most of the wanted words to be present so unrelated
    // devices of the same class still score 0.
    let coverage = token_coverage(want_trim, cand_trim);
    if coverage >= 0.6 {
        return 100 + (coverage * 100.0) as i32;
    }
    0
}

/// `true` for the synthetic "use the Windows default endpoint" UI entries.
pub fn is_default_device_entry(device_name: &str) -> bool {
    let t = device_name.trim();
    t.is_empty() || t.to_ascii_lowercase().starts_with("default")
}

/// Splits a `PWSTR` returned by COM into an owned `String` and frees the original allocation.
pub unsafe fn take_pwstr(ptr: PWSTR) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let out = ptr.to_string().ok();
    CoTaskMemFree(Some(ptr.0 as *const _));
    out
}

/// Reads an endpoint's `PKEY_Device_FriendlyName` through its property store.
pub unsafe fn endpoint_friendly_name(dev: &IMMDevice) -> Option<String> {
    let store = dev.OpenPropertyStore(STGM_READ).ok()?;
    let mut prop: PROPVARIANT = store.GetValue(&PKEY_Device_FriendlyName).ok()?;
    let name: Option<String> = PropVariantToBSTR(&prop)
        .ok()
        .and_then(|b: windows::core::BSTR| String::try_from(b).ok());
    let _ = PropVariantClear(&mut prop);
    name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty())
}

/// Resolves the best matching active endpoint for `flow`, falling back to the system default.
///
/// `want` is the friendly name the user picked in the UI; pass an empty string or any
/// `"Default …"` label to request the Windows default endpoint directly.
pub unsafe fn resolve_endpoint(
    enumerator: &IMMDeviceEnumerator,
    flow: EDataFlow,
    want: &str,
) -> Option<IMMDevice> {
    let default_endpoint = || -> Option<IMMDevice> {
        if flow == eCapture {
            enumerator
                .GetDefaultAudioEndpoint(eCapture, eCommunications)
                .or_else(|_| enumerator.GetDefaultAudioEndpoint(eCapture, eConsole))
                .ok()
        } else {
            enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()
        }
    };

    if is_default_device_entry(want) {
        return default_endpoint();
    }

    let mut best: Option<(i32, IMMDevice)> = None;
    if let Ok(collection) = enumerator.EnumAudioEndpoints(flow, DEVICE_STATE_ACTIVE) {
        let count = collection.GetCount().unwrap_or(0);
        for i in 0..count {
            let Ok(dev) = collection.Item(i) else { continue };
            let Some(name) = endpoint_friendly_name(&dev) else {
                continue;
            };
            let score = score_device_name(want, &name);
            if score > 0 && best.as_ref().map(|(b, _)| score > *b).unwrap_or(true) {
                best = Some((score, dev));
            }
        }
    }

    best.map(|(_, dev)| dev).or_else(default_endpoint)
}

/// Locates a running process by executable file name (case-insensitive).
///
/// Returns the **highest** matching PID as a heuristic for "most recently launched instance",
/// which is what a user means when they pick an application for Application Audio Capture.
pub fn find_pid_for_executable(exe: &str) -> Option<u32> {
    let want = exe.trim();
    if want.is_empty() {
        return None;
    }
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()?;
        let mut entry = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        let mut best: Option<u32> = None;
        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
                if name.eq_ignore_ascii_case(want) {
                    let pid = entry.th32ProcessID;
                    if best.map(|b| pid > b).unwrap_or(true) {
                        best = Some(pid);
                    }
                }
                entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
        best
    }
}

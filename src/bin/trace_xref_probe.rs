// Read-only scan for references to Mad Max's ray/terrain script API names.
// Usage: trace-xref-probe.exe <MadMax.exe PID> [--image-range <base-hex> <size-hex>]
// The process handle requests query and read access only; this tool never writes to it.
use std::ffi::c_void;
use std::mem::{size_of, zeroed};

#[cfg(not(windows))]
compile_error!("trace-xref-probe is Windows-only");

#[repr(C)]
struct ModuleEntry32W {
    size: u32,
    module_id: u32,
    process_id: u32,
    global_usage: u32,
    process_usage: u32,
    base_address: *mut u8,
    base_size: u32,
    module_handle: *mut c_void,
    module_name: [u16; 256],
    executable_path: [u16; 260],
}

#[repr(C)]
struct MemoryBasicInformation {
    base_address: *mut c_void,
    allocation_base: *mut c_void,
    allocation_protect: u32,
    partition_id: u16,
    region_size: usize,
    state: u32,
    protect: u32,
    kind: u32,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn OpenProcess(access: u32, inherit: i32, process_id: u32) -> *mut c_void;
    fn CloseHandle(handle: *mut c_void) -> i32;
    fn ReadProcessMemory(
        process: *mut c_void,
        address: *const c_void,
        buffer: *mut c_void,
        size: usize,
        read: *mut usize,
    ) -> i32;
    fn VirtualQueryEx(
        process: *mut c_void,
        address: *const c_void,
        information: *mut MemoryBasicInformation,
        length: usize,
    ) -> usize;
    fn CreateToolhelp32Snapshot(flags: u32, process_id: u32) -> *mut c_void;
    fn Module32FirstW(snapshot: *mut c_void, entry: *mut ModuleEntry32W) -> i32;
    fn Module32NextW(snapshot: *mut c_void, entry: *mut ModuleEntry32W) -> i32;
}

const PROCESS_QUERY_INFORMATION: u32 = 0x0400;
const PROCESS_VM_READ: u32 = 0x0010;
const TH32CS_SNAPMODULE: u32 = 0x0000_0008;
const TH32CS_SNAPMODULE32: u32 = 0x0000_0010;
const MEM_COMMIT: u32 = 0x1000;
const PAGE_NOACCESS: u32 = 0x01;
const PAGE_GUARD: u32 = 0x100;
const CHUNK_SIZE: usize = 1024 * 1024;
const TARGETS: [(&str, u32); 6] = [
    ("RaycastStaticFromCamera", 0x0126_EB68),
    ("RaycastStatic", 0x0126_EB80),
    ("ValidateTransformOnTerrainFirstHit", 0x0126_EC18),
    ("ValidateTransformOnTerrain", 0x0126_EC40),
    ("LowerCameraQueryHelper", 0x0252_EDF0),
    ("PointInBoundsPredicate", 0x0252_67E0),
];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(pid) = args.get(1).and_then(|arg| arg.parse::<u32>().ok()) else {
        eprintln!(
            "Usage: trace-xref-probe.exe <MadMax.exe PID> [--image-range <base-hex> <size-hex> | --peek|--peek-any <address> [length]]"
        );
        std::process::exit(2);
    };

    // Query and VM_READ are the only requested process rights.
    let process = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid) };
    if process.is_null() {
        eprintln!("Could not open PID {pid} for read-only inspection.");
        std::process::exit(1);
    }
    let peek_any = args.get(2).map(String::as_str) == Some("--peek-any");
    // Arbitrary peeks are useful for inspecting live stack buffers and do not
    // require module enumeration. Keep this mode read-only and bounded below.
    if let Some(address_text) = args
        .get(2)
        .filter(|arg| *arg == "--peek" || *arg == "--peek-any")
        .and_then(|_| args.get(3))
    {
        let address = usize::from_str_radix(address_text.trim_start_matches("0x"), 16).unwrap_or(0);
        let length = args
            .get(4)
            .and_then(|arg| arg.parse::<usize>().ok())
            .unwrap_or(64)
            .min(256);
        if peek_any {
            let mut bytes = vec![0u8; length];
            let mut read = 0usize;
            let success = unsafe {
                ReadProcessMemory(
                    process,
                    address as *const c_void,
                    bytes.as_mut_ptr().cast(),
                    length,
                    &mut read,
                )
            } != 0;
            unsafe { CloseHandle(process) };
            if !success || read == 0 {
                eprintln!("Could not read the requested range.");
                std::process::exit(1);
            }
            bytes.truncate(read);
            println!(
                "{address:#x}: {}",
                bytes
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            return;
        }
        let Some((image_base, image_end)) = find_main_module(pid) else {
            unsafe { CloseHandle(process) };
            eprintln!("Could not find MadMax.exe in PID {pid}.");
            std::process::exit(1);
        };
        if address < image_base || address.saturating_add(length) > image_end {
            eprintln!("Peek range is outside the MadMax.exe image.");
            unsafe { CloseHandle(process) };
            std::process::exit(2);
        }
        let mut bytes = vec![0u8; length];
        let mut read = 0usize;
        let success = unsafe {
            ReadProcessMemory(
                process,
                address as *const c_void,
                bytes.as_mut_ptr().cast(),
                length,
                &mut read,
            )
        } != 0;
        unsafe { CloseHandle(process) };
        if !success || read == 0 {
            eprintln!("Could not read the requested range.");
            std::process::exit(1);
        }
        bytes.truncate(read);
        println!(
            "{address:#x}: {}",
            bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
        return;
    }

    let image_range = if args.get(2).map(String::as_str) == Some("--image-range") {
        let parse_hex = |index: usize| {
            args.get(index)
                .map(|value| value.trim_start_matches("0x"))
                .and_then(|value| usize::from_str_radix(value, 16).ok())
        };
        parse_hex(3)
            .zip(parse_hex(4))
            .and_then(|(base, size)| base.checked_add(size).map(|end| (base, end)))
    } else {
        find_main_module(pid)
    };
    let Some((image_base, image_end)) = image_range else {
        unsafe { CloseHandle(process) };
        eprintln!("Could not determine MadMax.exe image range for PID {pid}.");
        std::process::exit(1);
    };

    println!("Read-only scan: PID {pid}, image {image_base:#x}..{image_end:#x}");
    let mut total_hits = 0;
    let mut cursor = image_base;
    while cursor < image_end {
        let mut info: MemoryBasicInformation = unsafe { zeroed() };
        let queried = unsafe {
            VirtualQueryEx(
                process,
                cursor as *const c_void,
                &mut info,
                size_of::<MemoryBasicInformation>(),
            )
        };
        if queried == 0 || info.region_size == 0 {
            cursor = cursor.saturating_add(0x1000);
            continue;
        }

        let region_start = (info.base_address as usize).max(image_base);
        let region_end = (info.base_address as usize)
            .saturating_add(info.region_size)
            .min(image_end);
        let readable = info.state == MEM_COMMIT
            && info.protect & PAGE_NOACCESS == 0
            && info.protect & PAGE_GUARD == 0;
        if readable && region_start < region_end {
            total_hits += scan_region(process, region_start, region_end, image_base);
        }
        cursor = (info.base_address as usize)
            .saturating_add(info.region_size)
            .max(cursor.saturating_add(0x1000));
    }
    unsafe { CloseHandle(process) };
    println!("Finished. Candidate references found: {total_hits}");
}

fn find_main_module(pid: u32) -> Option<(usize, usize)> {
    let snapshot =
        unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid) };
    if snapshot.is_null() || snapshot as isize == -1 {
        return None;
    }
    let mut entry: ModuleEntry32W = unsafe { zeroed() };
    entry.size = size_of::<ModuleEntry32W>() as u32;
    let mut found = None;
    let mut has_entry = unsafe { Module32FirstW(snapshot, &mut entry) } != 0;
    while has_entry {
        let name = String::from_utf16_lossy(&entry.module_name);
        if name
            .trim_end_matches('\0')
            .eq_ignore_ascii_case("MadMax.exe")
        {
            let base = entry.base_address as usize;
            found = Some((base, base.saturating_add(entry.base_size as usize)));
            break;
        }
        has_entry = unsafe { Module32NextW(snapshot, &mut entry) } != 0;
    }
    unsafe { CloseHandle(snapshot) };
    found
}

fn scan_region(process: *mut c_void, start: usize, end: usize, image_base: usize) -> usize {
    let mut hits = 0;
    let mut chunk_start = start;
    let mut carry = Vec::new();
    while chunk_start < end {
        let requested = (end - chunk_start).min(CHUNK_SIZE);
        let mut bytes = vec![0u8; requested];
        let mut read = 0usize;
        let success = unsafe {
            ReadProcessMemory(
                process,
                chunk_start as *const c_void,
                bytes.as_mut_ptr().cast(),
                requested,
                &mut read,
            )
        } != 0;
        if success && read > 0 {
            bytes.truncate(read);
            let combined_start = chunk_start.saturating_sub(carry.len());
            let mut combined = carry;
            combined.extend_from_slice(&bytes);
            hits += scan_bytes(combined_start, &combined, image_base);
            carry = combined[combined.len().saturating_sub(7)..].to_vec();
        } else {
            carry.clear();
        }
        chunk_start = chunk_start.saturating_add(requested);
    }
    hits
}

fn scan_bytes(start: usize, bytes: &[u8], image_base: usize) -> usize {
    let mut hits = 0;
    for (name, rva) in TARGETS {
        let target = image_base.saturating_add(rva as usize);
        for offset in 0..bytes.len().saturating_sub(3) {
            let displacement = i32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
            let next_ip = start.saturating_add(offset + 4);
            if (next_ip as isize).wrapping_add(displacement as isize) as usize == target {
                let context_start = offset.saturating_sub(8);
                let context_end = (offset + 12).min(bytes.len());
                let context = bytes[context_start..context_end]
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                println!(
                    "  possible RIP-relative xref to {name} at {:#x}: {context}",
                    start + offset
                );
                hits += 1;
            }
        }
        for offset in 0..bytes.len().saturating_sub(7) {
            let value = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap()) as usize;
            if value == target {
                println!("  absolute pointer xref to {name} at {:#x}", start + offset);
                hits += 1;
            }
        }
    }
    hits
}

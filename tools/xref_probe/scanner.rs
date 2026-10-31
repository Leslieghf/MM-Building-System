//! Bounded scanning for documented Mad Max code references.
use super::process::ReadOnlyProcess;

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

pub fn scan_image(process: &ReadOnlyProcess, image_start: usize, image_end: usize) -> usize {
    let mut hits = 0;
    let mut cursor = image_start;
    while cursor < image_end {
        let Some(region) = process.query(cursor) else {
            cursor = cursor.saturating_add(0x1000);
            continue;
        };
        let start = (region.base_address as usize).max(image_start);
        let end = (region.base_address as usize)
            .saturating_add(region.region_size)
            .min(image_end);
        if is_readable_committed(&region) && start < end {
            hits += scan_region(process, start, end, image_start);
        }
        cursor = (region.base_address as usize)
            .saturating_add(region.region_size)
            .max(cursor.saturating_add(0x1000));
    }
    hits
}

fn is_readable_committed(region: &super::process::MemoryBasicInformation) -> bool {
    region.state == MEM_COMMIT
        && region.protect & PAGE_NOACCESS == 0
        && region.protect & PAGE_GUARD == 0
}

fn scan_region(process: &ReadOnlyProcess, start: usize, end: usize, image_base: usize) -> usize {
    let mut hits = 0;
    let mut chunk_start = start;
    let mut carry = Vec::new();
    while chunk_start < end {
        let length = (end - chunk_start).min(CHUNK_SIZE);
        if let Ok(bytes) = process.read(chunk_start, length) {
            let combined_start = chunk_start.saturating_sub(carry.len());
            let mut combined = carry;
            combined.extend_from_slice(&bytes);
            hits += scan_bytes(combined_start, &combined, image_base);
            carry = combined[combined.len().saturating_sub(7)..].to_vec();
        } else {
            carry.clear();
        }
        chunk_start = chunk_start.saturating_add(length);
    }
    hits
}

fn scan_bytes(start: usize, bytes: &[u8], image_base: usize) -> usize {
    TARGETS
        .iter()
        .map(|(name, rva)| scan_target(start, bytes, image_base, name, *rva))
        .sum()
}

fn scan_target(start: usize, bytes: &[u8], image_base: usize, name: &str, rva: u32) -> usize {
    let target = image_base.saturating_add(rva as usize);
    let relative_hits = scan_relative_references(start, bytes, target, name);
    relative_hits + scan_absolute_references(start, bytes, target, name)
}

fn scan_relative_references(start: usize, bytes: &[u8], target: usize, name: &str) -> usize {
    let mut hits = 0;
    for offset in 0..bytes.len().saturating_sub(3) {
        let displacement = i32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        let next_ip = start.saturating_add(offset + 4);
        if (next_ip as isize).wrapping_add(displacement as isize) as usize == target {
            print_reference("possible RIP-relative", name, start + offset, bytes, offset);
            hits += 1;
        }
    }
    hits
}

fn scan_absolute_references(start: usize, bytes: &[u8], target: usize, name: &str) -> usize {
    let mut hits = 0;
    for offset in 0..bytes.len().saturating_sub(7) {
        let value = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap()) as usize;
        if value == target {
            println!("  absolute pointer xref to {name} at {:#x}", start + offset);
            hits += 1;
        }
    }
    hits
}

fn print_reference(kind: &str, name: &str, address: usize, bytes: &[u8], offset: usize) {
    let from = offset.saturating_sub(8);
    let to = (offset + 12).min(bytes.len());
    let context = bytes[from..to]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ");
    println!("  {kind} xref to {name} at {address:#x}: {context}");
}

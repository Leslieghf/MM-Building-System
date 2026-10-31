//! Read-only Windows process and module access for the xref scanner.
use std::ffi::c_void;
use std::mem::{size_of, zeroed};

const PROCESS_QUERY_INFORMATION: u32 = 0x0400;
const PROCESS_VM_READ: u32 = 0x0010;
const TH32CS_SNAPMODULE: u32 = 0x0000_0008;
const TH32CS_SNAPMODULE32: u32 = 0x0000_0010;

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
pub struct MemoryBasicInformation {
    pub base_address: *mut c_void,
    allocation_base: *mut c_void,
    allocation_protect: u32,
    partition_id: u16,
    pub region_size: usize,
    pub state: u32,
    pub protect: u32,
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

/// Process handle limited to query and virtual-memory read access.
pub struct ReadOnlyProcess(*mut c_void);

impl ReadOnlyProcess {
    /// Open a process without write, operation, or thread-creation rights.
    pub fn open(pid: u32) -> Result<Self, String> {
        let handle = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid) };
        if handle.is_null() {
            Err(format!(
                "Could not open PID {pid} for read-only inspection."
            ))
        } else {
            Ok(Self(handle))
        }
    }

    /// Copy up to `length` bytes from the target process.
    pub fn read(&self, address: usize, length: usize) -> Result<Vec<u8>, String> {
        let mut bytes = vec![0; length];
        let mut read = 0;
        let success = unsafe {
            ReadProcessMemory(
                self.0,
                address as *const c_void,
                bytes.as_mut_ptr().cast(),
                length,
                &mut read,
            )
        } != 0;
        if !success || read == 0 {
            return Err(format!("Could not read memory at {address:#x}."));
        }
        bytes.truncate(read);
        Ok(bytes)
    }

    /// Query the memory region containing `address`.
    pub fn query(&self, address: usize) -> Option<MemoryBasicInformation> {
        let mut information = unsafe { zeroed() };
        let queried = unsafe {
            VirtualQueryEx(
                self.0,
                address as *const c_void,
                &mut information,
                size_of::<MemoryBasicInformation>(),
            )
        };
        (queried != 0).then_some(information)
    }
}

impl Drop for ReadOnlyProcess {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

/// Return the main executable image range when the process module list is readable.
pub fn find_game_image(pid: u32) -> Option<(usize, usize)> {
    let snapshot =
        unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid) };
    if snapshot.is_null() || snapshot as isize == -1 {
        return None;
    }
    let range = find_module_in_snapshot(snapshot);
    unsafe { CloseHandle(snapshot) };
    range
}

fn find_module_in_snapshot(snapshot: *mut c_void) -> Option<(usize, usize)> {
    let mut entry: ModuleEntry32W = unsafe { zeroed() };
    entry.size = size_of::<ModuleEntry32W>() as u32;
    let mut has_entry = unsafe { Module32FirstW(snapshot, &mut entry) } != 0;
    while has_entry {
        if module_name(&entry).eq_ignore_ascii_case("MadMax.exe") {
            let base = entry.base_address as usize;
            return Some((base, base.saturating_add(entry.base_size as usize)));
        }
        has_entry = unsafe { Module32NextW(snapshot, &mut entry) } != 0;
    }
    None
}

fn module_name(entry: &ModuleEntry32W) -> String {
    String::from_utf16_lossy(&entry.module_name)
        .trim_end_matches('\0')
        .to_owned()
}

//! Read-only copies of the spawn handler's per-frame ID observations.
//!
//! The SDK describes these three `vector<CHashString>` fields at fixed offsets.
//! This module copies only bounded scalar IDs; it never retains handler or
//! vector pointers beyond the callback.
use super::super::*;
use crate::api::ecs::SpawnSystemFrameObservations;
use crate::api::game::abi::build::steam::SPAWN_SYSTEM_CONTEXT;
use crate::api::game::abi::game::spawn::{
    SDK_VECTOR_HEADER_BYTES, SPAWN_HANDLER_DESPAWNED_LAST_FRAME_OFFSET,
    SPAWN_HANDLER_FAILED_LAST_FRAME_OFFSET, SPAWN_HANDLER_SPAWNED_LAST_FRAME_OFFSET,
};

const MAX_IDS_PER_KIND_PER_FRAME: usize = 32;

pub(super) fn sample_spawn_system_events() -> SpawnSystemFrameObservations {
    let mut result = SpawnSystemFrameObservations::default();
    if SPAWN_SYSTEM_CONTEXT == 0 || !readable_span(SPAWN_SYSTEM_CONTEXT, size_of::<usize>()) {
        return result;
    }
    let handler = unsafe { ptr::read_volatile(SPAWN_SYSTEM_CONTEXT as *const usize) };
    if handler < 0x10000 {
        return result;
    }

    copy_hash_vector(
        handler,
        SPAWN_HANDLER_SPAWNED_LAST_FRAME_OFFSET,
        &mut result.spawned,
        &mut result.spawned_count,
    );
    copy_hash_vector(
        handler,
        SPAWN_HANDLER_DESPAWNED_LAST_FRAME_OFFSET,
        &mut result.despawned,
        &mut result.despawned_count,
    );
    copy_hash_vector(
        handler,
        SPAWN_HANDLER_FAILED_LAST_FRAME_OFFSET,
        &mut result.failed,
        &mut result.failed_count,
    );
    result
}

fn copy_hash_vector(
    handler: usize,
    field_offset: usize,
    output: &mut [u32; MAX_IDS_PER_KIND_PER_FRAME],
    output_count: &mut u8,
) {
    let Some(vector_address) = handler.checked_add(field_offset) else {
        return;
    };
    if !readable_span(vector_address, SDK_VECTOR_HEADER_BYTES) {
        return;
    }
    let begin = unsafe { ptr::read_unaligned(vector_address as *const usize) };
    let end = unsafe { ptr::read_unaligned((vector_address + size_of::<usize>()) as *const usize) };
    let capacity_end =
        unsafe { ptr::read_unaligned((vector_address + 2 * size_of::<usize>()) as *const usize) };
    if begin == 0 && end == 0 && capacity_end == 0 {
        return;
    }
    if begin < 0x10000 || end < begin || capacity_end < end {
        return;
    }
    let byte_length = end - begin;
    if byte_length % size_of::<u32>() != 0
        || byte_length > MAX_IDS_PER_KIND_PER_FRAME * size_of::<u32>()
        || !readable_span(begin, byte_length)
    {
        return;
    }
    let count = byte_length / size_of::<u32>();
    for (index, value) in output.iter_mut().take(count).enumerate() {
        *value = unsafe { ptr::read_unaligned((begin + index * size_of::<u32>()) as *const u32) };
    }
    *output_count = count as u8;
}

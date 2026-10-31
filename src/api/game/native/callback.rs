//! Post-simulation callback orchestration and schedule handoff.
//!
//! The installed native callback is the sole entry into game-phase sampling,
//! ECS execution, and effect draining. Sampling produces an owned snapshot;
//! the ECS schedule runs against that copy; accepted effects are then passed to
//! the adapter. The callback contains failures at the native boundary and
//! skips phases whose build or callback gates are not satisfied.
use super::*;

pub(super) fn trace_game_runtime_stage(stage: u32, description: &str) {
    let bit = 1_u32 << stage;
    if GAME_RUNTIME_TRACE_STAGES.fetch_or(bit, Ordering::AcqRel) & bit == 0 {
        append_log(&format!("Game callback diagnostic: {description}"));
    }
}

pub(super) fn run_bevy_game_frame(thread_id: u32, overlap_epoch: u64) {
    trace_game_runtime_stage(0, "entered runtime callback");
    if !callback_is_current(overlap_epoch) {
        return;
    }
    synchronize_game_input_focus(overlap_epoch);
    trace_game_runtime_stage(1, "input-focus synchronization returned");
    if sample_game_state_if_due(overlap_epoch) {
        trace_game_runtime_stage(2, "game-state probe returned");
    }
    if !callback_is_current(overlap_epoch) {
        return;
    }
    let snapshot = current_game_snapshot(thread_id);
    trace_game_runtime_stage(3, "game snapshot copied");
    run_game_runtime_frame(snapshot, overlap_epoch);
}

fn sample_game_state_if_due(overlap_epoch: u64) -> bool {
    let now = Instant::now();
    let mut last_sample = LAST_GAME_STATE_SAMPLE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if last_sample.is_some_and(|last| now.duration_since(last) < GAME_STATE_SAMPLE_INTERVAL) {
        return false;
    }
    if !callback_is_current(overlap_epoch) {
        return false;
    }
    *last_sample = Some(now);
    GAME_SAMPLE_SEQUENCE.fetch_add(1, Ordering::AcqRel);
    probe_game_state();
    true
}

fn callback_is_current(overlap_epoch: u64) -> bool {
    CALLBACK_IN_FLIGHT.load(Ordering::Acquire) == 1
        && CALLBACK_OVERLAP_EPOCH.load(Ordering::Acquire) == overlap_epoch
}

fn run_game_runtime_frame(snapshot: GameSnapshot, overlap_epoch: u64) {
    GAME_RUNTIME.with(|runtime_cell| {
        let mut runtime_cell = runtime_cell.borrow_mut();
        if runtime_cell.is_none()
            && let Some(endpoint) = take_runtime_endpoint()
        {
            *runtime_cell = Some(GameRuntime::new(endpoint));
            trace_game_runtime_stage(4, "Bevy game runtime initialized");
        }
        let Some(runtime) = runtime_cell.as_mut() else {
            return;
        };
        apply_pending_game_effect(runtime, snapshot, overlap_epoch);
        submit_script_raycast_requests(runtime, overlap_epoch);
        submit_script_debug_draws(runtime, overlap_epoch);
        trace_game_runtime_stage(5, "Bevy post-simulation schedule returned");
    });
}

fn submit_script_raycast_requests(runtime: &mut GameRuntime, epoch: u64) {
    for request in runtime.take_script_raycast_requests() {
        if !callback_is_current(epoch) {
            break;
        }
        let result = execute_script_raycast(request);
        runtime.complete_script_raycast(result);
    }
}

fn apply_pending_game_effect(runtime: &mut GameRuntime, snapshot: GameSnapshot, epoch: u64) {
    let Some(pending) = runtime.prepare_and_run(snapshot) else {
        return;
    };
    match pending {
        PendingGameIntent::Spawn(pending) => {
            let outcome = apply_spawn_intent(&pending, epoch);
            runtime.complete_spawn_intent(pending, outcome);
        }
        PendingGameIntent::Velocity(pending) => {
            let result = apply_velocity_intent(pending, epoch);
            runtime.complete_intent(pending, result);
        }
    }
}

fn submit_script_debug_draws(runtime: &mut GameRuntime, epoch: u64) {
    let draws = runtime.take_script_debug_draws();
    let Some(source) = draws.first().map(|effect| effect.origin.id.as_str()) else {
        return;
    };
    match draw_script_debug_primitives(&draws, epoch) {
        Err(reason) => append_log(&format!(
            "CDebugRenderer rejected script draw batch from {source}: {reason}."
        )),
        Ok(()) if !SCRIPT_DEBUG_DRAW_SUBMISSION_LOGGED.swap(true, Ordering::AcqRel) => {
            append_log(&format!(
                "CDebugRenderer accepted the first script draw batch from {source} ({} primitives). Visibility still depends on the game's renderer queue.",
                draws.len()
            ));
        }
        Ok(()) => {}
    }
}

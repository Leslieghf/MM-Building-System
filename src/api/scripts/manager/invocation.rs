//! One bounded Rhai callback invocation and persistent-state validation.
//!
//! # Contract
//!
//! Invocation limits are applied by the configured engine before user code
//! runs. The scope and callback-local collectors are discarded afterward;
//! only values accepted by the persistent-state validator survive. Errors are
//! returned to the manager as script events and do not unwind through the game
//! callback.
use super::*;

pub(super) fn invoke(
    engine: &Engine,
    sink: &ScriptEventSink,
    id: &ScriptId,
    script: &mut ActiveScript,
    function: &str,
    snapshot: GameSnapshot,
    request_id: Option<u64>,
    capacity: usize,
    effects: &mut Vec<VehicleVelocityEffect>,
    debug_capacity: usize,
    debug_work_capacity: usize,
    debug_effects: &mut Vec<ScriptDebugDrawEffect>,
    raycast_capacity: bool,
    raycast_tickets: RaycastTicketStore,
    raycast_effects: &mut Vec<ScriptRaycastRequest>,
) {
    let event = build_event(request_id, id, script, function);
    if script.status != ScriptStatus::Active {
        report_disabled_run(sink, event, script.status);
        return;
    }
    let log = InvocationLog::new(sink.clone(), event.clone());
    let write = InvocationVehicleWrite::new(
        script
            .capabilities
            .iter()
            .any(|id| id == "game.vehicle.write"),
        snapshot.vehicle_velocity.is_some(),
        effects.len() < capacity,
    );
    let debug = InvocationDebugDraw::new(
        script.capabilities.iter().any(|id| id == "debug.draw"),
        debug_capacity.min(MAX_DEBUG_DRAWS_PER_INVOCATION),
        debug_work_capacity.min(MAX_DEBUG_DRAW_WORK_PER_INVOCATION),
    );
    let raycast = InvocationRaycast::new(
        script.capabilities.iter().any(|id| id == "game.raycast"),
        raycast_capacity,
        event.clone(),
        snapshot.callback_sequence,
        raycast_tickets.clone(),
    );
    let context = GamePostSimContext::new_with_raycast(
        snapshot,
        log.clone(),
        &script.capabilities,
        write.clone(),
        raycast.clone(),
        debug.clone(),
    );
    let result = run_callback(engine, script, function, context);
    log.finish();
    let velocity_effect = write.finish();
    let raycast_request = raycast.finish();
    let debug_primitives = debug.finish();
    if let Some(failure) = callback_failure(script, &result) {
        report_callback_failure(sink, script, event, failure);
    } else {
        publish_callback_effects(
            sink,
            event,
            request_id,
            script.status,
            velocity_effect,
            debug_primitives,
            raycast_request,
            raycast_tickets,
            effects,
            debug_effects,
            raycast_effects,
        );
    }
}

fn build_event(
    request_id: Option<u64>,
    id: &ScriptId,
    script: &ActiveScript,
    function: &str,
) -> ScriptEvent {
    ScriptEvent {
        request_id,
        id: id.clone(),
        generation: script.generation,
        phase: ScriptPhaseId::GamePostSim,
        callback: function.to_owned(),
        kind: ScriptEventKind::Log,
        status: None,
        auto_reload: script.auto_reload,
        reload_error: script.reload_error.clone(),
        line: None,
        column: None,
        text: String::new(),
    }
}

fn run_callback(
    engine: &Engine,
    script: &mut ActiveScript,
    function: &str,
    context: GamePostSimContext,
) -> Result<Dynamic, Box<rhai::EvalAltResult>> {
    engine.call_fn_with_options::<Dynamic>(
        CallFnOptions::new().eval_ast(false),
        &mut script.scope,
        &script.ast,
        function,
        (context,),
    )
}

struct CallbackFailure {
    message: String,
    line: Option<usize>,
    column: Option<usize>,
}

fn callback_failure(
    script: &ActiveScript,
    result: &Result<Dynamic, Box<rhai::EvalAltResult>>,
) -> Option<CallbackFailure> {
    if let Err(error) = result {
        return Some(CallbackFailure {
            message: bounded_text(&error.to_string(), MAX_EVENT_BYTES),
            line: error.position().line(),
            column: error.position().position(),
        });
    }
    let state = script.scope.get_value::<Dynamic>("state");
    state
        .as_ref()
        .is_none_or(|state| {
            !state.is::<Map>() || !valid_state(state, 0, &mut MAX_STATE_VALUES.clone())
        })
        .then(|| CallbackFailure {
            message:
                "state must remain a bounded map of maps, arrays, strings, numbers, booleans and ()"
                    .to_owned(),
            line: None,
            column: None,
        })
}

fn report_disabled_run(sink: &ScriptEventSink, mut event: ScriptEvent, status: ScriptStatus) {
    if event.request_id.is_none() {
        return;
    }
    event.kind = ScriptEventKind::Error;
    event.status = Some(status);
    event.text = "on_run skipped: generation disabled by an earlier callback".to_owned();
    sink.emit(event);
}

fn report_callback_failure(
    sink: &ScriptEventSink,
    script: &mut ActiveScript,
    mut event: ScriptEvent,
    failure: CallbackFailure,
) {
    script.status = ScriptStatus::Disabled;
    // Release partially mutated state and retained context/log objects.
    script.scope.clear();
    event.kind = ScriptEventKind::Error;
    event.status = Some(ScriptStatus::Disabled);
    event.line = failure.line;
    event.column = failure.column;
    event.text = failure.message;
    sink.emit(event);
}

fn publish_callback_effects(
    sink: &ScriptEventSink,
    mut event: ScriptEvent,
    request_id: Option<u64>,
    status: ScriptStatus,
    velocity: Option<[f32; 3]>,
    primitives: Vec<DebugPrimitive>,
    raycast_request: Option<ScriptRaycastRequest>,
    raycast_tickets: RaycastTicketStore,
    effects: &mut Vec<VehicleVelocityEffect>,
    debug_effects: &mut Vec<ScriptDebugDrawEffect>,
    raycast_effects: &mut Vec<ScriptRaycastRequest>,
) {
    if let Some(velocity) = velocity {
        let mut origin = event.clone();
        origin.kind = ScriptEventKind::Status;
        origin.text = "vehicle velocity intent queued; awaiting native adapter".to_owned();
        sink.emit(origin.clone());
        effects.push(VehicleVelocityEffect { velocity, origin });
    }
    for primitive in primitives {
        debug_effects.push(ScriptDebugDrawEffect {
            primitive,
            origin: event.clone(),
        });
    }
    if let Some(request) = raycast_request {
        if raycast_tickets.insert_pending(&request) {
            if raycast_effects.len() < super::MAX_RAYCAST_REQUESTS_PER_PHASE {
                raycast_effects.push(request);
            } else {
                raycast_tickets.complete(ScriptRaycastCompletion {
                    ticket: request.ticket,
                    source: request.source,
                    available: false,
                    state_present: false,
                    fraction: None,
                    hit_position: None,
                });
            }
        } else {
            event.kind = ScriptEventKind::Error;
            event.text = "raycast ticket store full; request was discarded".to_owned();
            sink.emit(event.clone());
        }
    }
    if request_id.is_some() {
        event.kind = ScriptEventKind::Status;
        event.status = Some(status);
        event.text = "on_run completed".to_owned();
        sink.emit(event);
    }
}
fn valid_state(value: &Dynamic, depth: usize, remaining: &mut usize) -> bool {
    if depth > MAX_STATE_DEPTH || *remaining == 0 {
        return false;
    }
    *remaining -= 1;
    if value.is::<Map>() {
        value
            .clone_cast::<Map>()
            .values()
            .all(|v| valid_state(v, depth + 1, remaining))
    } else if value.is::<rhai::Array>() {
        value
            .clone_cast::<rhai::Array>()
            .iter()
            .all(|v| valid_state(v, depth + 1, remaining))
    } else {
        value.is_unit()
            || value.is::<bool>()
            || value.is::<rhai::INT>()
            || value.is::<rhai::FLOAT>()
            || value.is::<rhai::ImmutableString>()
            || value.is::<char>()
    }
}

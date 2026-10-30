//! One bounded Rhai callback invocation and persistent-state validation.
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
) {
    let mut event = ScriptEvent {
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
    };
    if script.status != ScriptStatus::Active {
        if request_id.is_some() {
            event.kind = ScriptEventKind::Error;
            event.status = Some(script.status);
            event.text = "on_run skipped: generation disabled by an earlier callback".to_owned();
            sink.emit(event);
        }
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
    let context = GamePostSimContext::new(
        snapshot,
        log.clone(),
        &script.capabilities,
        write.clone(),
        debug.clone(),
    );
    let result = engine.call_fn_with_options::<Dynamic>(
        CallFnOptions::new().eval_ast(false),
        &mut script.scope,
        &script.ast,
        function,
        (context,),
    );
    log.finish();
    let velocity_effect = write.finish();
    let debug_primitives = debug.finish();
    let error = match result {
        Err(error) => {
            event.line = error.position().line();
            event.column = error.position().position();
            Some(bounded_text(&error.to_string(), MAX_EVENT_BYTES))
        }
        Ok(_) => {
            let state = script.scope.get_value::<Dynamic>("state");
            if state.as_ref().is_none_or(|state| {
                !state.is::<Map>() || !valid_state(state, 0, &mut MAX_STATE_VALUES.clone())
            }) {
                Some("state must remain a bounded map of maps, arrays, strings, numbers, booleans and ()".to_owned())
            } else {
                None
            }
        }
    };
    if let Some(error) = error {
        script.status = ScriptStatus::Disabled;
        // Release any partially mutated state and retained context/log objects.
        script.scope.clear();
        event.kind = ScriptEventKind::Error;
        event.status = Some(ScriptStatus::Disabled);
        event.text = error;
        sink.emit(event);
    } else {
        if let Some(velocity) = velocity_effect {
            let mut origin = event.clone();
            origin.kind = ScriptEventKind::Status;
            origin.text = "vehicle velocity intent queued; awaiting native adapter".to_owned();
            sink.emit(origin.clone());
            effects.push(VehicleVelocityEffect { velocity, origin });
        }
        for primitive in debug_primitives {
            debug_effects.push(ScriptDebugDrawEffect {
                primitive,
                origin: event.clone(),
            });
        }
        if request_id.is_some() {
            event.kind = ScriptEventKind::Status;
            event.status = Some(script.status);
            event.text = "on_run completed".to_owned();
            sink.emit(event);
        }
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

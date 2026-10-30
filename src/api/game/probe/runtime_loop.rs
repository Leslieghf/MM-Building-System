//! Post-simulation callback orchestration and schedule handoff.
use super::*;

pub(super) fn trace_game_runtime_stage(stage: u32, description: &str) {
    let bit = 1_u32 << stage;
    if GAME_RUNTIME_TRACE_STAGES.fetch_or(bit, Ordering::AcqRel) & bit == 0 {
        append_log(&format!("Game callback diagnostic: {description}"));
    }
}

pub(super) fn run_bevy_game_frame(thread_id: u32, overlap_epoch: u64) {
    trace_game_runtime_stage(0, "entered runtime callback");
    if CALLBACK_IN_FLIGHT.load(Ordering::Acquire) != 1
        || CALLBACK_OVERLAP_EPOCH.load(Ordering::Acquire) != overlap_epoch
    {
        return;
    }
    synchronize_game_input_focus(overlap_epoch);
    trace_game_runtime_stage(1, "input-focus synchronization returned");
    let now = Instant::now();
    let should_sample = {
        let mut last_sample = LAST_GAME_STATE_SAMPLE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if last_sample
            .is_none_or(|last_sample| now.duration_since(last_sample) >= GAME_STATE_SAMPLE_INTERVAL)
        {
            *last_sample = Some(now);
            true
        } else {
            false
        }
    };
    if should_sample {
        if CALLBACK_IN_FLIGHT.load(Ordering::Acquire) != 1
            || CALLBACK_OVERLAP_EPOCH.load(Ordering::Acquire) != overlap_epoch
        {
            return;
        }
        GAME_SAMPLE_SEQUENCE.fetch_add(1, Ordering::AcqRel);
        probe_game_state();
        trace_game_runtime_stage(2, "game-state probe returned");
    }

    if CALLBACK_IN_FLIGHT.load(Ordering::Acquire) != 1
        || CALLBACK_OVERLAP_EPOCH.load(Ordering::Acquire) != overlap_epoch
    {
        return;
    }

    let snapshot = current_game_snapshot(thread_id);
    trace_game_runtime_stage(3, "game snapshot copied");
    GAME_RUNTIME.with(|runtime_cell| {
        let mut runtime_cell = runtime_cell.borrow_mut();
        if runtime_cell.is_none() {
            if let Some(endpoint) = take_runtime_endpoint() {
                *runtime_cell = Some(GameRuntime::new(endpoint));
                trace_game_runtime_stage(4, "Bevy game runtime initialized");
            }
        }
        let Some(runtime) = runtime_cell.as_mut() else {
            return;
        };

        if let Some(pending) = runtime.prepare_and_run(snapshot) {
            match pending {
                PendingGameIntent::Spawn(pending) => {
                    let outcome = apply_spawn_intent(&pending, overlap_epoch);
                    runtime.complete_spawn_intent(pending, outcome);
                }
                PendingGameIntent::Velocity(pending) => {
                    let result = apply_velocity_intent(pending, overlap_epoch);
                    runtime.complete_intent(pending, result);
                }
            }
        }
        let script_debug_draws = runtime.take_script_debug_draws();
        if !script_debug_draws.is_empty() {
            if let Err(reason) = draw_script_debug_primitives(&script_debug_draws, overlap_epoch) {
                let source = script_debug_draws
                    .first()
                    .map(|effect| effect.origin.id.as_str())
                    .unwrap_or("unknown");
                append_log(&format!(
                    "CDebugRenderer rejected script draw batch from {source}: {reason}."
                ));
            } else if !SCRIPT_DEBUG_DRAW_SUBMISSION_LOGGED.swap(true, Ordering::AcqRel) {
                let source = script_debug_draws
                    .first()
                    .map(|effect| effect.origin.id.as_str())
                    .unwrap_or("unknown");
                append_log(&format!(
                    "CDebugRenderer accepted the first script draw batch from {source} ({} primitives). Visibility still depends on the game's renderer queue.",
                    script_debug_draws.len()
                ));
            }
        }
        trace_game_runtime_stage(5, "Bevy post-simulation schedule returned");
    });
}

pub(super) fn draw_script_debug_primitives(
    draws: &[crate::scripting::manager::ScriptDebugDrawEffect],
    overlap_epoch: u64,
) -> Result<(), &'static str> {
    if CALLBACK_IN_FLIGHT.load(Ordering::Acquire) != 1
        || CALLBACK_OVERLAP_EPOCH.load(Ordering::Acquire) != overlap_epoch
    {
        return Err("UpdatePostSim callback was not exclusive");
    }
    let module = unsafe { GetModuleHandleW(hudhook::windows::core::PCWSTR::null()) }
        .map_err(|_| "could not read the game image base")?;
    let image_base = module.0 as usize;
    if image_base != STEAM_IMAGE_BASE {
        return Err("game image base did not match the supported Steam build");
    }
    let facade_global = image_base + CDEBUG_RENDERER_INSTANCE_GLOBAL_RVA;
    if !readable_span(facade_global, size_of::<*mut c_void>()) {
        return Err("CDebugRenderer singleton pointer is unreadable");
    }
    let facade = unsafe { ptr::read_volatile(facade_global as *const *mut c_void) };
    if facade.is_null() || !readable_span(facade as usize, 0x18) {
        return Err("CDebugRenderer facade object is unavailable");
    }
    let facade_vtable = unsafe { ptr::read_volatile(facade.cast::<usize>()) };
    if facade_vtable != image_base + CDEBUG_RENDERER_FACADE_VTABLE_RVA
        || !readable_span(
            facade_vtable,
            (CDEBUG_RENDERER_BOUNDS_CONVEX_VERTICES_SLOT + 1) * size_of::<usize>(),
        )
    {
        return Err("CDebugRenderer facade vtable did not match the mapped interface");
    }
    let backend =
        unsafe { ptr::read_volatile((facade as *const u8).add(0x10).cast::<*mut c_void>()) };
    if backend.is_null() || !readable_span(backend as usize, size_of::<usize>()) {
        return Err("CDebugRenderer backend is unavailable");
    }
    let backend_vtable = unsafe { ptr::read_volatile(backend.cast::<usize>()) };
    if !readable_span(
        backend_vtable,
        (CDEBUG_RENDERER_BOUNDS_CONVEX_VERTICES_SLOT + 1) * size_of::<usize>(),
    ) {
        return Err("CDebugRenderer backend vtable is unreadable");
    }
    let backend_line = unsafe {
        ptr::read_volatile((backend_vtable as *const usize).add(CDEBUG_RENDERER_LINE_SLOT))
    };
    let backend_arrow = unsafe {
        ptr::read_volatile((backend_vtable as *const usize).add(CDEBUG_RENDERER_ARROW_SLOT))
    };
    let backend_axes = unsafe {
        ptr::read_volatile((backend_vtable as *const usize).add(CDEBUG_RENDERER_AXES_SLOT))
    };
    if backend_line != image_base + PRIMITIVE_RENDERER_LINE_RVA
        || backend_arrow != image_base + PRIMITIVE_RENDERER_ARROW_RVA
        || backend_axes != image_base + PRIMITIVE_RENDERER_AXES_RVA
        || !is_readable_executable_page(backend_line)
        || !is_readable_executable_page(backend_arrow)
        || !is_readable_executable_page(backend_axes)
    {
        return Err("active backend did not match the mapped NGraphicsEngine primitive renderer");
    }
    let line_target = unsafe {
        ptr::read_volatile((facade_vtable as *const usize).add(CDEBUG_RENDERER_LINE_SLOT))
    };
    let arrow_target = unsafe {
        ptr::read_volatile((facade_vtable as *const usize).add(CDEBUG_RENDERER_ARROW_SLOT))
    };
    let axes_target = unsafe {
        ptr::read_volatile((facade_vtable as *const usize).add(CDEBUG_RENDERER_AXES_SLOT))
    };
    if line_target != image_base + CDEBUG_RENDERER_LINE_METHOD_RVA
        || arrow_target != image_base + CDEBUG_RENDERER_ARROW_METHOD_RVA
        || axes_target != image_base + CDEBUG_RENDERER_AXES_METHOD_RVA
        || !is_readable_executable_page(line_target)
        || !is_readable_executable_page(arrow_target)
        || !is_readable_executable_page(axes_target)
    {
        return Err("CDebugRenderer facade methods did not match the mapped line/arrow/axes slots");
    }

    let mapped_facade_methods = [
        (
            CDEBUG_RENDERER_SPHERE_SLOT,
            CDEBUG_RENDERER_SPHERE_METHOD_RVA,
        ),
        (
            CDEBUG_RENDERER_SPARSE_SPHERE_SLOT,
            CDEBUG_RENDERER_SPARSE_SPHERE_METHOD_RVA,
        ),
        (
            CDEBUG_RENDERER_TESSELLATED_SPHERE_SLOT,
            CDEBUG_RENDERER_TESSELLATED_SPHERE_METHOD_RVA,
        ),
        (CDEBUG_RENDERER_AABB_SLOT, CDEBUG_RENDERER_AABB_METHOD_RVA),
        (
            CDEBUG_RENDERER_TRANSFORMED_AABB_SLOT,
            CDEBUG_RENDERER_TRANSFORMED_AABB_METHOD_RVA,
        ),
        (
            CDEBUG_RENDERER_QUEUED_TRANSFORMED_AABB_SLOT,
            CDEBUG_RENDERER_QUEUED_TRANSFORMED_AABB_METHOD_RVA,
        ),
        (
            CDEBUG_RENDERER_BOUNDS_SPHERE_SLOT,
            CDEBUG_RENDERER_BOUNDS_SPHERE_METHOD_RVA,
        ),
        (
            CDEBUG_RENDERER_BOUNDS_CYLINDER_SLOT,
            CDEBUG_RENDERER_BOUNDS_CYLINDER_METHOD_RVA,
        ),
        (
            CDEBUG_RENDERER_BOUNDS_CAPSULE_SLOT,
            CDEBUG_RENDERER_BOUNDS_CAPSULE_METHOD_RVA,
        ),
        (
            CDEBUG_RENDERER_BOUNDS_CONVEX_VERTICES_SLOT,
            CDEBUG_RENDERER_BOUNDS_CONVEX_VERTICES_METHOD_RVA,
        ),
    ];
    for (slot, rva) in mapped_facade_methods {
        let target = unsafe { ptr::read_volatile((facade_vtable as *const usize).add(slot)) };
        if target != image_base + rva || !is_readable_executable_page(target) {
            return Err("CDebugRenderer facade geometry method did not match the mapped interface");
        }
    }
    let backend_geometry_slots = [
        CDEBUG_RENDERER_SPHERE_SLOT,
        CDEBUG_RENDERER_SPARSE_SPHERE_SLOT,
        CDEBUG_RENDERER_TESSELLATED_SPHERE_SLOT,
        CDEBUG_RENDERER_AABB_SLOT,
        CDEBUG_RENDERER_TRANSFORMED_AABB_SLOT,
        CDEBUG_RENDERER_QUEUED_TRANSFORMED_AABB_SLOT,
        CDEBUG_RENDERER_BOUNDS_SPHERE_SLOT,
        CDEBUG_RENDERER_BOUNDS_CYLINDER_SLOT,
        CDEBUG_RENDERER_BOUNDS_CAPSULE_SLOT,
        CDEBUG_RENDERER_BOUNDS_CONVEX_VERTICES_SLOT,
    ];
    if backend_geometry_slots.into_iter().any(|slot| {
        let target = unsafe { ptr::read_volatile((backend_vtable as *const usize).add(slot)) };
        !is_readable_executable_page(target)
    }) {
        return Err("CDebugRenderer backend geometry slot was not executable");
    }

    let line: CDebugRendererLine = unsafe { std::mem::transmute(line_target) };
    let arrow: CDebugRendererArrow = unsafe { std::mem::transmute(arrow_target) };
    let axes: CDebugRendererAxes = unsafe { std::mem::transmute(axes_target) };
    let sphere: CDebugRendererSphere =
        unsafe { std::mem::transmute(image_base + CDEBUG_RENDERER_SPHERE_METHOD_RVA) };
    let sparse_sphere: CDebugRendererSphere =
        unsafe { std::mem::transmute(image_base + CDEBUG_RENDERER_SPARSE_SPHERE_METHOD_RVA) };
    let tessellated_sphere: CDebugRendererSphere =
        unsafe { std::mem::transmute(image_base + CDEBUG_RENDERER_TESSELLATED_SPHERE_METHOD_RVA) };
    let aabb: CDebugRendererAabb =
        unsafe { std::mem::transmute(image_base + CDEBUG_RENDERER_AABB_METHOD_RVA) };
    let transformed_aabb: CDebugRendererTransformedAabb =
        unsafe { std::mem::transmute(image_base + CDEBUG_RENDERER_TRANSFORMED_AABB_METHOD_RVA) };
    let queued_transformed_aabb: CDebugRendererTransformedAabb = unsafe {
        std::mem::transmute(image_base + CDEBUG_RENDERER_QUEUED_TRANSFORMED_AABB_METHOD_RVA)
    };
    let bounds_sphere: CDebugRendererBoundsShape =
        unsafe { std::mem::transmute(image_base + CDEBUG_RENDERER_BOUNDS_SPHERE_METHOD_RVA) };
    let bounds_cylinder: CDebugRendererBoundsShape =
        unsafe { std::mem::transmute(image_base + CDEBUG_RENDERER_BOUNDS_CYLINDER_METHOD_RVA) };
    let bounds_capsule: CDebugRendererBoundsShape =
        unsafe { std::mem::transmute(image_base + CDEBUG_RENDERER_BOUNDS_CAPSULE_METHOD_RVA) };
    let bounds_convex_vertices: CDebugRendererBoundsShape = unsafe {
        std::mem::transmute(image_base + CDEBUG_RENDERER_BOUNDS_CONVEX_VERTICES_METHOD_RVA)
    };
    for draw in draws {
        match draw.primitive {
            DebugPrimitive::Axes { position, length } => {
                let mut transform = [0.0_f32; 16];
                transform[0] = 1.0;
                transform[5] = 1.0;
                transform[10] = 1.0;
                transform[15] = 1.0;
                transform[12] = position[0];
                transform[13] = position[1];
                transform[14] = position[2];
                unsafe { axes(facade, &transform, length) };
            }
            DebugPrimitive::TransformedAxes { transform, length } => {
                unsafe { axes(facade, &transform, length) };
            }
            DebugPrimitive::Line { start, end, color } => {
                unsafe { line(facade, &start, &end, &color, &color) };
            }
            DebugPrimitive::Arrow { start, end, color } => {
                unsafe { arrow(facade, &start, &end, &color, &color) };
            }
            DebugPrimitive::Sphere {
                center,
                radius,
                color,
            } => unsafe { sphere(facade, &center, radius, &color) },
            DebugPrimitive::SparseSphere {
                center,
                radius,
                color,
            } => unsafe { sparse_sphere(facade, &center, radius, &color) },
            DebugPrimitive::TessellatedSphere {
                center,
                radius,
                color,
            } => unsafe { tessellated_sphere(facade, &center, radius, &color) },
            DebugPrimitive::Aabb {
                minimum,
                maximum,
                color,
            } => unsafe { aabb(facade, &minimum, &maximum, &color) },
            DebugPrimitive::TransformedAabb {
                transform,
                minimum,
                maximum,
                color,
            } => unsafe { transformed_aabb(facade, &transform, &minimum, &maximum, &color) },
            DebugPrimitive::QueuedTransformedAabb {
                transform,
                minimum,
                maximum,
                color,
            } => unsafe { queued_transformed_aabb(facade, &transform, &minimum, &maximum, &color) },
            DebugPrimitive::BoundsSphere {
                transform,
                minimum,
                maximum,
                color,
            } => unsafe { bounds_sphere(facade, &transform, &minimum, &maximum, &color) },
            DebugPrimitive::BoundsCylinder {
                transform,
                minimum,
                maximum,
                color,
            } => unsafe { bounds_cylinder(facade, &transform, &minimum, &maximum, &color) },
            DebugPrimitive::BoundsCapsule {
                transform,
                minimum,
                maximum,
                color,
            } => unsafe { bounds_capsule(facade, &transform, &minimum, &maximum, &color) },
            DebugPrimitive::BoundsConvexVertices {
                transform,
                minimum,
                maximum,
                color,
            } => unsafe { bounds_convex_vertices(facade, &transform, &minimum, &maximum, &color) },
        }
    }
    Ok(())
}

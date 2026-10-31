//! Build-specific `CDebugRenderer` submission for validated script primitives.
//!
//! The adapter resolves the renderer and its supported method slots only after
//! the executable/build gate passes. It consumes owned primitives drained from
//! callback collectors; this layer performs the native calls, while validation
//! and per-callback work budgets are enforced before submission. Renderer
//! behavior remains experimental and is not guaranteed by a successful build.
use super::super::*;

#[derive(Clone, Copy)]
struct RendererContext {
    image_base: usize,
    facade: *mut c_void,
}

#[derive(Clone, Copy)]
struct RendererMethods {
    line: CDebugRendererLine,
    arrow: CDebugRendererArrow,
    axes: CDebugRendererAxes,
    sphere: CDebugRendererSphere,
    sparse_sphere: CDebugRendererSphere,
    tessellated_sphere: CDebugRendererSphere,
    aabb: CDebugRendererAabb,
    transformed_aabb: CDebugRendererTransformedAabb,
    queued_transformed_aabb: CDebugRendererTransformedAabb,
    solid_cube: CDebugRendererBoundsShape,
    solid_arrow: CDebugRendererBoundsShape,
}

pub(in crate::api::game::native) fn draw_script_debug_primitives(
    draws: &[crate::api::scripts::manager::ScriptDebugDrawEffect],
    overlap_epoch: u64,
) -> Result<(), &'static str> {
    ensure_exclusive_callback(overlap_epoch)?;
    let context = inspect_renderer()?;
    let methods = resolve_renderer_methods(context.image_base)?;
    for draw in draws {
        submit_primitive(context.facade, methods, draw.primitive);
    }
    Ok(())
}

fn ensure_exclusive_callback(epoch: u64) -> Result<(), &'static str> {
    if CALLBACK_IN_FLIGHT.load(Ordering::Acquire) == 1
        && CALLBACK_OVERLAP_EPOCH.load(Ordering::Acquire) == epoch
    {
        Ok(())
    } else {
        Err("UpdatePostSim callback was not exclusive")
    }
}

fn inspect_renderer() -> Result<RendererContext, &'static str> {
    let image_base = supported_image_base()?;
    let facade = read_facade(image_base)?;
    let facade_vtable = validate_facade_vtable(image_base, facade)?;
    let backend_vtable = read_backend_vtable(facade)?;
    validate_backend_primitives(image_base, backend_vtable)?;
    validate_facade_geometry(image_base, facade_vtable)?;
    validate_backend_geometry(backend_vtable)?;
    Ok(RendererContext { image_base, facade })
}

fn supported_image_base() -> Result<usize, &'static str> {
    let module = unsafe { GetModuleHandleW(hudhook::windows::core::PCWSTR::null()) }
        .map_err(|_| "could not read the game image base")?;
    let image_base = module.0 as usize;
    if image_base == STEAM_IMAGE_BASE {
        Ok(image_base)
    } else {
        Err("game image base did not match the supported Steam build")
    }
}

fn read_facade(image_base: usize) -> Result<*mut c_void, &'static str> {
    let address = image_base + CDEBUG_RENDERER_INSTANCE_GLOBAL_RVA;
    if !readable_span(address, size_of::<*mut c_void>()) {
        return Err("CDebugRenderer singleton pointer is unreadable");
    }
    let facade = unsafe { ptr::read_volatile(address as *const *mut c_void) };
    if facade.is_null() || !readable_span(facade as usize, 0x18) {
        return Err("CDebugRenderer facade object is unavailable");
    }
    Ok(facade)
}

fn validate_facade_vtable(image_base: usize, facade: *mut c_void) -> Result<usize, &'static str> {
    let vtable = unsafe { ptr::read_volatile(facade.cast::<usize>()) };
    let required_bytes = (CDEBUG_RENDERER_SOLID_ARROW_SLOT + 1) * size_of::<usize>();
    if vtable != image_base + CDEBUG_RENDERER_FACADE_VTABLE_RVA
        || !readable_span(vtable, required_bytes)
    {
        return Err("CDebugRenderer facade vtable did not match the mapped interface");
    }
    Ok(vtable)
}

fn read_backend_vtable(facade: *mut c_void) -> Result<usize, &'static str> {
    let backend =
        unsafe { ptr::read_volatile((facade as *const u8).add(0x10).cast::<*mut c_void>()) };
    if backend.is_null() || !readable_span(backend as usize, size_of::<usize>()) {
        return Err("CDebugRenderer backend is unavailable");
    }
    let vtable = unsafe { ptr::read_volatile(backend.cast::<usize>()) };
    let required_bytes = (CDEBUG_RENDERER_SOLID_ARROW_SLOT + 1) * size_of::<usize>();
    if !readable_span(vtable, required_bytes) {
        return Err("CDebugRenderer backend vtable is unreadable");
    }
    Ok(vtable)
}

fn validate_backend_primitives(image_base: usize, vtable: usize) -> Result<(), &'static str> {
    let methods = [
        (
            CDEBUG_RENDERER_LINE_SLOT,
            image_base + PRIMITIVE_RENDERER_LINE_RVA,
        ),
        (
            CDEBUG_RENDERER_ARROW_SLOT,
            image_base + PRIMITIVE_RENDERER_ARROW_RVA,
        ),
        (
            CDEBUG_RENDERER_AXES_SLOT,
            image_base + PRIMITIVE_RENDERER_AXES_RVA,
        ),
    ];
    for (slot, expected) in methods {
        let target = unsafe { ptr::read_volatile((vtable as *const usize).add(slot)) };
        if target != expected || !is_readable_executable_page(target) {
            return Err("active backend did not match the mapped primitive renderer");
        }
    }
    Ok(())
}

fn validate_facade_geometry(image_base: usize, vtable: usize) -> Result<(), &'static str> {
    let methods = [
        (CDEBUG_RENDERER_LINE_SLOT, CDEBUG_RENDERER_LINE_METHOD_RVA),
        (CDEBUG_RENDERER_ARROW_SLOT, CDEBUG_RENDERER_ARROW_METHOD_RVA),
        (CDEBUG_RENDERER_AXES_SLOT, CDEBUG_RENDERER_AXES_METHOD_RVA),
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
            CDEBUG_RENDERER_SOLID_CUBE_SLOT,
            CDEBUG_RENDERER_SOLID_CUBE_METHOD_RVA,
        ),
        (
            CDEBUG_RENDERER_SOLID_ARROW_SLOT,
            CDEBUG_RENDERER_SOLID_ARROW_METHOD_RVA,
        ),
    ];
    for (slot, rva) in methods {
        let target = unsafe { ptr::read_volatile((vtable as *const usize).add(slot)) };
        if target != image_base + rva || !is_readable_executable_page(target) {
            return Err("CDebugRenderer facade method did not match the mapped interface");
        }
    }
    Ok(())
}

fn validate_backend_geometry(vtable: usize) -> Result<(), &'static str> {
    let slots = [
        CDEBUG_RENDERER_SPHERE_SLOT,
        CDEBUG_RENDERER_SPARSE_SPHERE_SLOT,
        CDEBUG_RENDERER_TESSELLATED_SPHERE_SLOT,
        CDEBUG_RENDERER_AABB_SLOT,
        CDEBUG_RENDERER_TRANSFORMED_AABB_SLOT,
        CDEBUG_RENDERER_QUEUED_TRANSFORMED_AABB_SLOT,
        CDEBUG_RENDERER_SOLID_CUBE_SLOT,
        CDEBUG_RENDERER_SOLID_ARROW_SLOT,
    ];
    let executable = slots.into_iter().all(|slot| {
        let target = unsafe { ptr::read_volatile((vtable as *const usize).add(slot)) };
        is_readable_executable_page(target)
    });
    executable
        .then_some(())
        .ok_or("CDebugRenderer backend geometry slot was not executable")
}

fn resolve_renderer_methods(image_base: usize) -> Result<RendererMethods, &'static str> {
    let address = |rva| image_base + rva;
    Ok(unsafe {
        RendererMethods {
            line: std::mem::transmute(address(CDEBUG_RENDERER_LINE_METHOD_RVA)),
            arrow: std::mem::transmute(address(CDEBUG_RENDERER_ARROW_METHOD_RVA)),
            axes: std::mem::transmute(address(CDEBUG_RENDERER_AXES_METHOD_RVA)),
            sphere: std::mem::transmute(address(CDEBUG_RENDERER_SPHERE_METHOD_RVA)),
            sparse_sphere: std::mem::transmute(address(CDEBUG_RENDERER_SPARSE_SPHERE_METHOD_RVA)),
            tessellated_sphere: std::mem::transmute(address(
                CDEBUG_RENDERER_TESSELLATED_SPHERE_METHOD_RVA,
            )),
            aabb: std::mem::transmute(address(CDEBUG_RENDERER_AABB_METHOD_RVA)),
            transformed_aabb: std::mem::transmute(address(
                CDEBUG_RENDERER_TRANSFORMED_AABB_METHOD_RVA,
            )),
            queued_transformed_aabb: std::mem::transmute(address(
                CDEBUG_RENDERER_QUEUED_TRANSFORMED_AABB_METHOD_RVA,
            )),
            solid_cube: std::mem::transmute(address(CDEBUG_RENDERER_SOLID_CUBE_METHOD_RVA)),
            solid_arrow: std::mem::transmute(address(CDEBUG_RENDERER_SOLID_ARROW_METHOD_RVA)),
        }
    })
}

fn submit_primitive(facade: *mut c_void, methods: RendererMethods, primitive: DebugPrimitive) {
    match primitive {
        DebugPrimitive::Axes { position, length } => {
            let transform = identity_transform_at(position);
            unsafe { (methods.axes)(facade, &transform, length) };
        }
        DebugPrimitive::TransformedAxes { transform, length } => unsafe {
            (methods.axes)(facade, &transform, length)
        },
        DebugPrimitive::Line { start, end, color } => unsafe {
            (methods.line)(facade, &start, &end, &color, &color)
        },
        DebugPrimitive::Arrow { start, end, color } => unsafe {
            (methods.arrow)(facade, &start, &end, &color, &color)
        },
        shape => submit_geometry(facade, methods, shape),
    }
}

fn identity_transform_at(position: [f32; 3]) -> [f32; 16] {
    let mut transform = [0.0_f32; 16];
    transform[0] = 1.0;
    transform[5] = 1.0;
    transform[10] = 1.0;
    transform[15] = 1.0;
    transform[12] = position[0];
    transform[13] = position[1];
    transform[14] = position[2];
    transform
}

fn submit_geometry(facade: *mut c_void, methods: RendererMethods, primitive: DebugPrimitive) {
    match primitive {
        DebugPrimitive::WireSphere { .. }
        | DebugPrimitive::XraySphere { .. }
        | DebugPrimitive::SurfaceWireSphere { .. } => submit_sphere(facade, methods, primitive),
        DebugPrimitive::WireAabb { .. }
        | DebugPrimitive::WireBox { .. }
        | DebugPrimitive::XrayWireBox { .. } => submit_box(facade, methods, primitive),
        DebugPrimitive::SolidCube { .. } | DebugPrimitive::SolidArrow { .. } => {
            submit_solid_shape(facade, methods, primitive)
        }
        DebugPrimitive::Axes { .. }
        | DebugPrimitive::TransformedAxes { .. }
        | DebugPrimitive::Line { .. }
        | DebugPrimitive::Arrow { .. } => {
            unreachable!("simple primitive routed to geometry dispatch")
        }
    }
}

fn submit_sphere(facade: *mut c_void, methods: RendererMethods, primitive: DebugPrimitive) {
    match primitive {
        DebugPrimitive::WireSphere {
            center,
            radius,
            color,
        } => unsafe { (methods.sphere)(facade, &center, radius, &color) },
        DebugPrimitive::XraySphere {
            center,
            radius,
            color,
        } => unsafe { (methods.sparse_sphere)(facade, &center, radius, &color) },
        DebugPrimitive::SurfaceWireSphere {
            center,
            radius,
            color,
        } => unsafe { (methods.tessellated_sphere)(facade, &center, radius, &color) },
        _ => unreachable!("non-sphere primitive routed to sphere dispatch"),
    }
}

fn submit_box(facade: *mut c_void, methods: RendererMethods, primitive: DebugPrimitive) {
    match primitive {
        DebugPrimitive::WireAabb {
            minimum,
            maximum,
            color,
        } => unsafe { (methods.aabb)(facade, &minimum, &maximum, &color) },
        DebugPrimitive::WireBox {
            transform,
            minimum,
            maximum,
            color,
        } => unsafe { (methods.transformed_aabb)(facade, &transform, &minimum, &maximum, &color) },
        DebugPrimitive::XrayWireBox {
            transform,
            minimum,
            maximum,
            color,
        } => unsafe {
            (methods.queued_transformed_aabb)(facade, &transform, &minimum, &maximum, &color)
        },
        _ => unreachable!("non-box primitive routed to box dispatch"),
    }
}

fn submit_solid_shape(facade: *mut c_void, methods: RendererMethods, primitive: DebugPrimitive) {
    match primitive {
        DebugPrimitive::SolidCube {
            transform,
            minimum,
            maximum,
            color,
        } => unsafe { (methods.solid_cube)(facade, &transform, &minimum, &maximum, &color) },
        DebugPrimitive::SolidArrow {
            transform,
            minimum,
            maximum,
            color,
        } => unsafe { (methods.solid_arrow)(facade, &transform, &minimum, &maximum, &color) },
        _ => unreachable!("non-solid shape routed to solid shape dispatch"),
    }
}

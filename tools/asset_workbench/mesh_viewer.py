"""Build a self-contained, flat-shaded WebGL preview for decoded static meshes."""

from __future__ import annotations

import json
import math
import struct
from array import array
from pathlib import Path


def viewer_markup(mesh_data: dict[str, object]) -> str:
    meshes = mesh_data.get("decoded_geometry") or []
    if not meshes:
        return ""
    payload = json.dumps(meshes, separators=(",", ":"), ensure_ascii=True)
    payload = payload.replace("<", "\\u003c").replace(">", "\\u003e").replace("&", "\\u0026")
    return f"""<section class="mesh-viewer">
<div class="mesh-viewer-title">3D geometry · flat shaded · textures and lighting not applied</div>
<canvas id="mesh-canvas" aria-label="Interactive 3D mesh preview"></canvas>
<div class="mesh-viewer-help">Drag to orbit · wheel to zoom · double-click to reset</div>
</section>
<script>
const MESH_DATA={payload};
(()=>{{
const canvas=document.getElementById('mesh-canvas');
const gl=canvas.getContext('webgl',{{antialias:true,alpha:false}});
const status=document.getElementById('mesh-status');
if(!gl){{canvas.replaceWith(Object.assign(document.createElement('div'),{{className:'mesh-error',textContent:'WebGL is unavailable in this browser.'}}));return;}}
const vertexSource='attribute vec3 aPosition; uniform mat4 uMvp; void main(){{gl_Position=uMvp*vec4(aPosition,1.0);}}';
const fragmentSource='precision mediump float; uniform vec3 uColor; void main(){{gl_FragColor=vec4(uColor,1.0);}}';
function shader(type,source){{const s=gl.createShader(type);gl.shaderSource(s,source);gl.compileShader(s);if(!gl.getShaderParameter(s,gl.COMPILE_STATUS))throw new Error(gl.getShaderInfoLog(s));return s;}}
const program=gl.createProgram();gl.attachShader(program,shader(gl.VERTEX_SHADER,vertexSource));gl.attachShader(program,shader(gl.FRAGMENT_SHADER,fragmentSource));gl.linkProgram(program);if(!gl.getProgramParameter(program,gl.LINK_STATUS))throw new Error(gl.getProgramInfoLog(program));
const positionLocation=gl.getAttribLocation(program,'aPosition');const mvpLocation=gl.getUniformLocation(program,'uMvp');const colorLocation=gl.getUniformLocation(program,'uColor');
const points=MESH_DATA.flatMap(m=>m.positions);const low=[Infinity,Infinity,Infinity],high=[-Infinity,-Infinity,-Infinity];for(const p of points)for(let i=0;i<3;i++){{low[i]=Math.min(low[i],p[i]);high[i]=Math.max(high[i],p[i]);}}
const center=low.map((v,i)=>(v+high[i])*0.5);const radius=Math.max(...high.map((v,i)=>v-low[i]))*0.65||1;
const prepared=MESH_DATA.map(m=>{{const positions=new Float32Array(m.positions.length*3);m.positions.forEach((p,i)=>{{positions[i*3]=p[0]-center[0];positions[i*3+1]=p[1]-center[1];positions[i*3+2]=p[2]-center[2];}});const vb=gl.createBuffer();gl.bindBuffer(gl.ARRAY_BUFFER,vb);gl.bufferData(gl.ARRAY_BUFFER,positions,gl.STATIC_DRAW);const ib=gl.createBuffer();gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,ib);gl.bufferData(gl.ELEMENT_ARRAY_BUFFER,new Uint16Array(m.indices),gl.STATIC_DRAW);return{{vb,ib,count:m.indices.length,color:[[0.76,0.54,0.29],[0.42,0.68,0.71],[0.72,0.66,0.42],[0.72,0.46,0.39]][m.mesh_index%4]}};}});
function multiply(a,b){{const o=new Float32Array(16);for(let c=0;c<4;c++)for(let r=0;r<4;r++)o[c*4+r]=a[r]*b[c*4]+a[4+r]*b[c*4+1]+a[8+r]*b[c*4+2]+a[12+r]*b[c*4+3];return o;}}
function perspective(fov,aspect,near,far){{const f=1/Math.tan(fov/2),nf=1/(near-far),o=new Float32Array(16);o[0]=f/aspect;o[5]=f;o[10]=(far+near)*nf;o[11]=-1;o[14]=2*far*near*nf;return o;}}
function translate(x,y,z){{const o=new Float32Array(16);o[0]=o[5]=o[10]=o[15]=1;o[12]=x;o[13]=y;o[14]=z;return o;}}
function rotateX(a){{const c=Math.cos(a),s=Math.sin(a),o=new Float32Array(16);o[0]=o[15]=1;o[5]=c;o[6]=s;o[9]=-s;o[10]=c;return o;}}
function rotateY(a){{const c=Math.cos(a),s=Math.sin(a),o=new Float32Array(16);o[0]=c;o[2]=-s;o[5]=1;o[8]=s;o[10]=c;o[15]=1;return o;}}
let yaw=0.65,pitch=-0.2,distance=radius*3.2,drag=null;
function draw(){{const dpr=window.devicePixelRatio||1,w=Math.max(1,Math.floor(canvas.clientWidth*dpr)),h=Math.max(1,Math.floor(canvas.clientHeight*dpr));if(canvas.width!==w||canvas.height!==h){{canvas.width=w;canvas.height=h;}}gl.viewport(0,0,w,h);gl.enable(gl.DEPTH_TEST);gl.clearColor(0.055,0.078,0.098,1);gl.clear(gl.COLOR_BUFFER_BIT|gl.DEPTH_BUFFER_BIT);gl.useProgram(program);const proj=perspective(0.72,w/h,0.01,Math.max(10000,distance*100));const view=translate(0,0,-distance);const model=multiply(rotateX(pitch),rotateY(yaw));const mvp=multiply(proj,multiply(view,model));gl.uniformMatrix4fv(mvpLocation,false,mvp);gl.enableVertexAttribArray(positionLocation);for(const mesh of prepared){{gl.bindBuffer(gl.ARRAY_BUFFER,mesh.vb);gl.vertexAttribPointer(positionLocation,3,gl.FLOAT,false,0,0);gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,mesh.ib);gl.uniform3fv(colorLocation,mesh.color);gl.drawElements(gl.TRIANGLES,mesh.count,gl.UNSIGNED_SHORT,0);}}requestAnimationFrame(draw);}}
canvas.addEventListener('pointerdown',e=>{{drag=[e.clientX,e.clientY];canvas.setPointerCapture(e.pointerId);}});canvas.addEventListener('pointermove',e=>{{if(!drag)return;yaw+=(e.clientX-drag[0])*0.008;pitch=Math.max(-1.5,Math.min(1.5,pitch+(e.clientY-drag[1])*0.008));drag=[e.clientX,e.clientY];}});canvas.addEventListener('pointerup',()=>drag=null);canvas.addEventListener('pointercancel',()=>drag=null);canvas.addEventListener('wheel',e=>{{e.preventDefault();distance=Math.max(radius*0.5,Math.min(radius*30,distance*Math.exp(e.deltaY*0.001)));}},{{passive:false}});canvas.addEventListener('dblclick',()=>{{yaw=0.65;pitch=-0.2;distance=radius*3.2;}});window.addEventListener('resize',()=>{{canvas.width=0;}});draw();
}})();
</script>"""


def write_glb(mesh_data: dict[str, object], name: str) -> bytes:
    """Package decoded positions, UVs, normals, and triangle indices as a GLB 2.0."""
    geometry = mesh_data.get("decoded_geometry") or []
    if not geometry:
        raise ValueError("No decoded meshes are available for GLB export")

    binary = bytearray()
    buffer_views: list[dict[str, int]] = []
    accessors: list[dict[str, object]] = []
    materials: list[dict[str, object]] = []
    meshes: list[dict[str, object]] = []

    def append_buffer(payload: bytes, target: int) -> int:
        while len(binary) % 4:
            binary.append(0)
        view_index = len(buffer_views)
        offset = len(binary)
        binary.extend(payload)
        buffer_views.append({"buffer": 0, "byteOffset": offset, "byteLength": len(payload), "target": target})
        return view_index

    def add_accessor(view: int, component_type: int, count: int, kind: str, **extra: object) -> int:
        accessor: dict[str, object] = {
            "bufferView": view,
            "componentType": component_type,
            "count": count,
            "type": kind,
        }
        accessor.update(extra)
        accessors.append(accessor)
        return len(accessors) - 1

    palette = ((0.76, 0.54, 0.29, 1.0), (0.42, 0.68, 0.71, 1.0), (0.72, 0.66, 0.42, 1.0), (0.72, 0.46, 0.39, 1.0))
    for mesh in geometry:
        positions = mesh["positions"]
        uvs = mesh["primary_uvs"]
        indices = mesh["indices"]
        vertex_count = len(positions)
        if not vertex_count or not indices:
            continue

        low = [min(position[axis] for position in positions) for axis in range(3)]
        high = [max(position[axis] for position in positions) for axis in range(3)]
        normals = [[0.0, 0.0, 0.0] for _ in range(vertex_count)]
        for triangle in range(0, len(indices) - 2, 3):
            ia, ib, ic = indices[triangle : triangle + 3]
            a, b, c = positions[ia], positions[ib], positions[ic]
            ab = (b[0] - a[0], b[1] - a[1], b[2] - a[2])
            ac = (c[0] - a[0], c[1] - a[1], c[2] - a[2])
            normal = (
                ab[1] * ac[2] - ab[2] * ac[1],
                ab[2] * ac[0] - ab[0] * ac[2],
                ab[0] * ac[1] - ab[1] * ac[0],
            )
            for vertex in (ia, ib, ic):
                for axis in range(3):
                    normals[vertex][axis] += normal[axis]
        normalized_normals = []
        for normal in normals:
            length = math.sqrt(sum(axis * axis for axis in normal))
            normalized_normals.append(tuple(axis / length for axis in normal) if length > 1e-12 else (0.0, 1.0, 0.0))

        position_data = array("f", (value for position in positions for value in position))
        normal_data = array("f", (value for normal in normalized_normals for value in normal))
        uv_data = array("f", (value for uv in uvs for value in uv))
        index_data = array("H", indices)
        if struct.pack("=I", 1)[0] != 1:
            for values in (position_data, normal_data, uv_data, index_data):
                values.byteswap()

        position_accessor = add_accessor(
            append_buffer(position_data.tobytes(), 34962),
            5126,
            vertex_count,
            "VEC3",
            min=low,
            max=high,
        )
        normal_accessor = add_accessor(
            append_buffer(normal_data.tobytes(), 34962), 5126, vertex_count, "VEC3"
        )
        uv_accessor = add_accessor(append_buffer(uv_data.tobytes(), 34962), 5126, vertex_count, "VEC2")
        index_accessor = add_accessor(
            append_buffer(index_data.tobytes(), 34963),
            5123,
            len(indices),
            "SCALAR",
            min=[min(indices)],
            max=[max(indices)],
        )
        material_index = len(materials)
        material_name = f"mesh_{mesh['mesh_index']}"
        materials.append(
            {
                "name": material_name,
                "pbrMetallicRoughness": {
                    "baseColorFactor": list(palette[material_index % len(palette)]),
                    "metallicFactor": 0.0,
                    "roughnessFactor": 1.0,
                },
                "doubleSided": True,
            }
        )
        meshes.append(
            {
                "name": material_name,
                "primitives": [
                    {
                        "attributes": {
                            "POSITION": position_accessor,
                            "NORMAL": normal_accessor,
                            "TEXCOORD_0": uv_accessor,
                        },
                        "indices": index_accessor,
                        "material": material_index,
                        "mode": 4,
                    }
                ],
            }
        )

    if not meshes:
        raise ValueError("The decoded model contains no nonempty mesh primitives")

    document = {
        "asset": {"version": "2.0", "generator": "Mad Max Asset Workbench"},
        "scene": 0,
        "scenes": [{"nodes": list(range(len(meshes)))}],
        "nodes": [{"mesh": index, "name": mesh["name"]} for index, mesh in enumerate(meshes)],
        "meshes": meshes,
        "materials": materials,
        "accessors": accessors,
        "bufferViews": buffer_views,
        "buffers": [{"byteLength": len(binary)}],
    }
    json_chunk = json.dumps(document, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    json_chunk += b" " * ((-len(json_chunk)) % 4)
    bin_chunk = bytes(binary) + b"\0" * ((-len(binary)) % 4)
    total_length = 12 + 8 + len(json_chunk) + 8 + len(bin_chunk)
    return (
        struct.pack("<4sII", b"glTF", 2, total_length)
        + struct.pack("<I4s", len(json_chunk), b"JSON")
        + json_chunk
        + struct.pack("<I4s", len(bin_chunk), b"BIN\0")
        + bin_chunk
    )


def write_thumbnail(mesh_data: dict[str, object], output_path: Path, size: int = 256) -> bool:
    """Rasterize a small transparent flat-shaded catalogue thumbnail offline."""
    try:
        from PIL import Image
    except ImportError:
        return False

    meshes = mesh_data.get("decoded_geometry") or []
    points = [position for mesh in meshes for position in mesh["positions"]]
    triangle_count = sum(len(mesh["indices"]) // 3 for mesh in meshes)
    if not points or triangle_count > 100_000:
        return False

    minimum = [min(position[axis] for position in points) for axis in range(3)]
    maximum = [max(position[axis] for position in points) for axis in range(3)]
    center = [(minimum[axis] + maximum[axis]) * 0.5 for axis in range(3)]
    radius = max(maximum[axis] - minimum[axis] for axis in range(3)) or 1.0
    yaw, pitch = 0.65, -0.2
    cy, sy = math.cos(yaw), math.sin(yaw)
    cx, sx = math.cos(pitch), math.sin(pitch)

    def transform(position: tuple[float, float, float]) -> tuple[float, float, float]:
        x, y, z = (position[axis] - center[axis] for axis in range(3))
        rx, rz = cy * x + sy * z, -sy * x + cy * z
        ry, rz = cx * y - sx * rz, sx * y + cx * rz
        return rx, ry, rz

    projected_meshes = []
    margin_scale = size * 0.76 / radius
    depth_buffer = [-float("inf")] * (size * size)
    pixels = bytearray(size * size * 4)
    palette = ((194, 138, 74), (107, 174, 184), (184, 169, 107), (184, 117, 100))
    light = (0.32, 0.82, 0.47)
    light_length = math.sqrt(sum(axis * axis for axis in light))
    light = tuple(axis / light_length for axis in light)

    for mesh_index, mesh in enumerate(meshes):
        transformed = [transform(tuple(position)) for position in mesh["positions"]]
        screen = [
            (size * 0.5 + position[0] * margin_scale, size * 0.5 - position[1] * margin_scale, position[2])
            for position in transformed
        ]
        projected_meshes.append((mesh_index, mesh, transformed, screen))

    for mesh_index, mesh, transformed, screen in projected_meshes:
        base_color = palette[mesh_index % len(palette)]
        indices = mesh["indices"]
        for start in range(0, len(indices) - 2, 3):
            ia, ib, ic = indices[start : start + 3]
            a, b, c = transformed[ia], transformed[ib], transformed[ic]
            ab, ac = (b[0] - a[0], b[1] - a[1], b[2] - a[2]), (c[0] - a[0], c[1] - a[1], c[2] - a[2])
            normal = (
                ab[1] * ac[2] - ab[2] * ac[1],
                ab[2] * ac[0] - ab[0] * ac[2],
                ab[0] * ac[1] - ab[1] * ac[0],
            )
            normal_length = math.sqrt(sum(axis * axis for axis in normal))
            if normal_length <= 1e-12:
                continue
            illumination = 0.36 + 0.64 * abs(sum(normal[i] * light[i] for i in range(3)) / normal_length)
            color = tuple(min(255, int(channel * illumination)) for channel in base_color)
            vertices = (screen[ia], screen[ib], screen[ic])
            x0 = max(0, int(min(vertex[0] for vertex in vertices)))
            x1 = min(size - 1, int(max(vertex[0] for vertex in vertices)) + 1)
            y0 = max(0, int(min(vertex[1] for vertex in vertices)))
            y1 = min(size - 1, int(max(vertex[1] for vertex in vertices)) + 1)
            area = (vertices[1][0] - vertices[0][0]) * (vertices[2][1] - vertices[0][1]) - (
                vertices[1][1] - vertices[0][1]
            ) * (vertices[2][0] - vertices[0][0])
            if abs(area) < 1e-9:
                continue
            for y in range(y0, y1 + 1):
                py = y + 0.5
                for x in range(x0, x1 + 1):
                    px = x + 0.5
                    w0 = ((vertices[1][0] - px) * (vertices[2][1] - py) - (vertices[1][1] - py) * (vertices[2][0] - px)) / area
                    w1 = ((vertices[2][0] - px) * (vertices[0][1] - py) - (vertices[2][1] - py) * (vertices[0][0] - px)) / area
                    w2 = 1.0 - w0 - w1
                    if w0 < -1e-6 or w1 < -1e-6 or w2 < -1e-6:
                        continue
                    depth = w0 * vertices[0][2] + w1 * vertices[1][2] + w2 * vertices[2][2]
                    pixel_index = y * size + x
                    if depth <= depth_buffer[pixel_index]:
                        continue
                    depth_buffer[pixel_index] = depth
                    pixel_offset = pixel_index * 4
                    pixels[pixel_offset : pixel_offset + 4] = bytes((*color, 255))

    output_path.parent.mkdir(parents=True, exist_ok=True)
    Image.frombytes("RGBA", (size, size), bytes(pixels)).save(output_path, format="PNG", optimize=True)
    return True

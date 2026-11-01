"""Build a self-contained, searchable HTML browser for the local asset manifest."""

from __future__ import annotations

import json
import pathlib


ROOT = pathlib.Path("target/content-index")
MANIFEST = ROOT / "asset-workbench-resources.jsonl"
OUTPUT = ROOT / "asset-workbench.html"


def compact_record(record: dict[str, object]) -> dict[str, object]:
    profile = record.get("entity_profile") or {}
    preview = record.get("preview") or {}
    archive = record.get("archive") or {}
    members = record.get("compiled_member") or {}
    references = record.get("references") or {}
    return {
        "id": record["resource_id"],
        "path": record["path"],
        "path_kind": record["path_kind"],
        "ext": record["extension"],
        "families": record["families"],
        "archive": bool(archive.get("present")),
        "archive_sets": archive.get("sets", []),
        "member_owners": len(members.get("owners", [])),
        "references": references.get("count", 0),
        "classes": profile.get("direct_root_classes", []),
        "root_class": profile.get("root_classification"),
        "catalog": record.get("catalog_entries", []),
        "preview": preview,
        "image": preview.get("image"),
        "model_export": preview.get("model_export"),
        "structure": preview.get("structure"),
    }


def main() -> int:
    if not MANIFEST.is_file():
        raise SystemExit(f"Missing {MANIFEST}; run build_manifest.py first.")

    records = [
        compact_record(json.loads(line))
        for line in MANIFEST.open(encoding="utf-8")
        if line.strip()
    ]
    data = json.dumps(records, separators=(",", ":"), ensure_ascii=True)
    data = data.replace("<", "\\u003c").replace(">", "\\u003e").replace("&", "\\u0026")
    page = TEMPLATE.replace("__ASSET_DATA__", data)
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(page, encoding="utf-8")
    print(json.dumps({"written": str(OUTPUT.resolve()), "records": len(records), "bytes": OUTPUT.stat().st_size}, indent=2))
    return 0


TEMPLATE = r"""<!doctype html>
<html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Mad Max Asset Workbench</title>
<style>
:root{color-scheme:dark;--bg:#101820;--panel:#17232e;--panel2:#0d151d;--line:#304251;--text:#dce6ee;--muted:#8fa4b5;--blue:#315d78;--green:#55c77b;--yellow:#e8bd4f;--red:#ec685e}
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--text);font:12px/1.4 system-ui,Segoe UI,sans-serif;height:100vh;overflow:hidden}
header{height:48px;background:#14212c;border-bottom:1px solid var(--line);display:flex;align-items:center;gap:12px;padding:0 14px}header strong{font-size:14px;letter-spacing:.04em}header .count{color:var(--muted)}
main{height:calc(100vh - 48px);display:grid;grid-template-columns:210px minmax(350px,1fr) 340px;gap:7px;padding:7px}
aside,.results,.detail{min-height:0;background:var(--panel);border:1px solid var(--line);border-radius:4px}aside{padding:8px;overflow:auto}.filters{display:grid;gap:3px}.filter{border:0;border-radius:3px;padding:6px 7px;background:transparent;color:var(--text);text-align:left;font:inherit;cursor:pointer}.filter:hover,.filter.active{background:#244155}.filter span{float:right;color:var(--muted)}.section{margin:12px 0 5px;color:#91a9bc;font-size:10px;font-weight:700;letter-spacing:.08em;text-transform:uppercase}
.center{display:flex;min-height:0;flex-direction:column;gap:6px}.toolbar{display:flex;align-items:center;gap:6px}.toolbar input,.toolbar select{height:30px;background:var(--panel2);border:1px solid var(--line);border-radius:3px;color:var(--text);padding:0 8px;font:inherit}.toolbar input{flex:1;min-width:0}.results{overflow:auto;flex:1}.row{min-height:38px;display:grid;grid-template-columns:minmax(0,1fr) 110px 96px;gap:8px;align-items:center;padding:5px 9px;border-bottom:1px solid #263744;cursor:pointer}.row:nth-child(even){background:#14202a}.row:hover,.row.selected{background:#203746}.path{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.meta{color:var(--muted);font-size:10px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.badge{display:inline-flex;align-items:center;gap:5px;white-space:nowrap;font-size:10px;color:#b6c5d0}.dot{width:7px;height:7px;border-radius:50%;background:#708392}.dot.green{background:var(--green)}.dot.yellow{background:var(--yellow)}.dot.red{background:var(--red)}.pager{height:28px;display:flex;align-items:center;justify-content:center;gap:12px;color:var(--muted)}button{font:inherit;color:var(--text);background:#203746;border:1px solid var(--line);border-radius:3px;padding:4px 9px;cursor:pointer}button:hover{background:#2c5068}.detail{padding:12px;overflow:auto}.detail h2{font-size:14px;margin:0 0 5px;overflow-wrap:anywhere}.detail .fullpath{font:11px/1.5 ui-monospace,Consolas,monospace;color:#a9bdcb;overflow-wrap:anywhere}.chips{display:flex;flex-wrap:wrap;gap:4px;margin:10px 0}.chip{padding:2px 5px;border:1px solid #3b5263;border-radius:3px;color:#b9c9d5;font-size:10px}.kv{display:grid;grid-template-columns:100px 1fr;gap:6px 8px;margin-top:12px}.kv .k{color:var(--muted)}.kv div{overflow-wrap:anywhere}.actions{display:flex;gap:6px;margin-top:12px}.status{margin-top:8px;padding:6px 7px;border-left:3px solid #617584;background:var(--panel2)}.status.green{border-color:var(--green)}.status.yellow{border-color:var(--yellow)}.status.red{border-color:var(--red)}a{color:#83c8f1;text-decoration:none}a:hover{text-decoration:underline}.empty{padding:20px;color:var(--muted)}
.row{grid-template-columns:44px minmax(0,1fr) 110px 96px;min-height:48px;padding-top:4px;padding-bottom:4px}.row-thumb,.row-placeholder{width:40px;height:38px;object-fit:contain;background:#0d151d;border:1px solid #304251;border-radius:3px}.row-placeholder{display:grid;place-items:center;color:#60798b;font-weight:700}.detail-image{display:block;width:100%;max-height:260px;object-fit:contain;background:#0d151d;border:1px solid var(--line);margin:10px 0}.structure{white-space:pre-line;font-size:11px;color:#c0d1de;background:#0d151d;border:1px solid var(--line);padding:7px;margin-top:10px;line-height:1.6}
@media(max-width:1000px){main{grid-template-columns:165px minmax(280px,1fr)}.detail{display:none}}@media(max-width:650px){main{grid-template-columns:1fr}.filters{display:flex;overflow:auto}.filter{white-space:nowrap}aside{max-height:115px}.section{display:none}}
</style>
<header><strong>ASSET WORKBENCH</strong><span class="count" id="summary">Loading local inventory…</span></header>
<main><aside><div class="section">Resource families</div><div class="filters" id="filters"></div><div class="meta">Family totals can overlap.</div><div class="section">Spawn status</div><div class="badge"><i class="dot green"></i>Spawn observed</div><div class="badge"><i class="dot yellow"></i>Experimental spawn route</div><div class="badge"><i class="dot red"></i>No known spawn route</div></aside>
<section class="center"><div class="toolbar"><input id="search" placeholder="Search paths, extensions, classes…"><select id="preview"><option value="all">All resources</option><option value="yes">Preview prepared</option><option value="no">No preview yet</option></select><select id="spawn"><option value="all">Any spawn status</option><option value="verified">Spawn observed</option><option value="candidate">Experimental route</option><option value="blocked">No known route</option></select></div><div class="results" id="results"></div><div class="pager"><button id="previous">‹</button><span id="page">0 results</span><button id="next">›</button></div></section><section class="detail" id="detail"><div class="empty">Select a resource to inspect its inventory links and available local preview.</div></section></main>
<script>
const DATA=__ASSET_DATA__;const PAGE_SIZE=200;let family="all",page=0,selected=null,filtered=[];
const familyNames={entity_definition:"Entity definitions",entity_package:"Entity packages",model_candidate:"Models / geometry",texture_candidate:"Textures",script_candidate:"Scripts",graph_or_behavior_candidate:"Graphs / behavior",animation_or_rig_candidate:"Animation / rigs",audio_candidate:"Audio",effect_or_vfx_candidate:"Effects / VFX",collision_or_effect_candidate:"Collision / physics",world_or_streaming_candidate:"World / streaming",vehicle_configuration_candidate:"Vehicle configuration",unknown:"Unclassified"};
const familyCounts=new Map();for(const r of DATA)for(const f of r.families)familyCounts.set(f,(familyCounts.get(f)||0)+1);
const filterBox=document.getElementById("filters");function addFilter(id,label,count){const b=document.createElement("button");b.className="filter"+(family===id?" active":"");b.innerHTML="";const n=document.createElement("span");n.textContent=count.toLocaleString();b.append(document.createTextNode(label),n);b.onclick=()=>{family=id;page=0;drawFilters();applyFilters()};filterBox.append(b)}
function drawFilters(){filterBox.replaceChildren();addFilter("all","All resources",DATA.length);for(const [id,label] of Object.entries(familyNames))if(familyCounts.has(id))addFilter(id,label,familyCounts.get(id))}drawFilters();
function statusOf(r){if(!r.catalog.length)return null;if(r.catalog.some(x=>x.readiness==="verified"))return ["green","Spawn observed"];if(r.catalog.some(x=>x.readiness==="candidate"))return ["yellow","Experimental spawn route"];return ["red","No known spawn route"]}
function applyFilters(){const q=document.getElementById("search").value.trim().toLocaleLowerCase();const pv=document.getElementById("preview").value;const sp=document.getElementById("spawn").value;filtered=DATA.filter(r=>{if(family!=="all"&&!r.families.includes(family))return false;if(pv==="yes"&&!r.preview.report)return false;if(pv==="no"&&r.preview.report)return false;const s=statusOf(r);if(sp!=="all"&&(!s||s[0]!==({verified:"green",candidate:"yellow",blocked:"red"}[sp])))return false;if(q){const hay=[r.path,r.id,r.ext,...r.families,...r.classes,...r.catalog.map(c=>`${c.name} ${c.id} ${c.kind}`)].join(" ").toLocaleLowerCase();if(!hay.includes(q))return false}return true});page=Math.min(page,Math.max(0,Math.ceil(filtered.length/PAGE_SIZE)-1));renderRows();document.getElementById("summary").textContent=`${filtered.length.toLocaleString()} shown · ${DATA.length.toLocaleString()} indexed resources`}
function renderRows(){const host=document.getElementById("results");host.replaceChildren();const start=page*PAGE_SIZE;for(const r of filtered.slice(start,start+PAGE_SIZE)){const row=document.createElement("div");row.className="row"+(selected===r.id?" selected":"");if(r.image){const image=document.createElement("img");image.className="row-thumb";image.src=r.image.replace("target/content-index/","");image.alt="";row.append(image)}else{const placeholder=document.createElement("div");placeholder.className="row-placeholder";placeholder.textContent=(r.ext||"?").replace(".","").slice(0,3).toUpperCase();row.append(placeholder)}const path=document.createElement("div");path.className="path";path.textContent=r.path||r.id;const meta=document.createElement("div");meta.className="meta";meta.textContent=r.families.map(f=>familyNames[f]||f).join(" · ");const badge=document.createElement("div");badge.className="badge";const s=statusOf(r);if(s){const dot=document.createElement("i");dot.className="dot "+s[0];badge.append(dot,document.createTextNode(s[1]))}else badge.textContent=r.preview.report?"Preview ready":r.archive?"In game files":"Indexed";row.append(path,meta,badge);row.onclick=()=>{selected=r.id;renderRows();renderDetail(r)};host.append(row)}document.getElementById("page").textContent=filtered.length?`${start+1}–${Math.min(start+PAGE_SIZE,filtered.length)} of ${filtered.length.toLocaleString()}`:"0 results";document.getElementById("previous").disabled=page===0;document.getElementById("next").disabled=(page+1)*PAGE_SIZE>=filtered.length}
function renderDetail(r){const d=document.getElementById("detail");d.replaceChildren();const h=document.createElement("h2");h.textContent=r.path||r.id;d.append(h);const full=document.createElement("div");full.className="fullpath";full.textContent=r.path_kind==="basename"?`Basename-only entry · ${r.path}`:r.path;d.append(full);const chips=document.createElement("div");chips.className="chips";for(const f of r.families){const c=document.createElement("span");c.className="chip";c.textContent=familyNames[f]||f;chips.append(c)}d.append(chips);const s=statusOf(r);if(s){const box=document.createElement("div");box.className="status "+s[0];box.textContent=s[1];d.append(box)}if(r.image){const image=document.createElement("img");image.className="detail-image";image.src=r.image.replace("target/content-index/","");image.alt=r.structure&&r.structure.geometry_preview?"Transparent flat-shaded model thumbnail":"Decoded texture preview";d.append(image)}const kv=document.createElement("div");kv.className="kv";const values=[["Extension",r.ext||"(none)"],["Archive",r.archive?`Present · ${r.archive_sets.join(", ")}`:"Not confirmed"],["Bundle owners",r.member_owners||0],["Entity refs",r.references||0],["Root classes",r.classes.length?r.classes.join(", "):r.root_class||"—"],["Catalog entries",r.catalog.map(c=>c.name).join(", ")||"—"]];for(const [k,v] of values){const a=document.createElement("div");a.className="k";a.textContent=k;const b=document.createElement("div");b.textContent=String(v);kv.append(a,b)}d.append(kv);if(r.structure){const mesh=document.createElement("div");mesh.className="structure";mesh.textContent=`${r.structure.format}: ${r.structure.block_count} blocks, ${r.structure.vertex_count.toLocaleString()} vertices, ${r.structure.index_count.toLocaleString()} indices\n${r.structure.decoder_status}`;d.append(mesh)}if(r.preview.report){const actions=document.createElement("div");actions.className="actions";const a=document.createElement("a");a.textContent="Open prepared preview";a.href=r.preview.report.replace("target/content-index/","");a.target="_blank";a.rel="noreferrer";actions.append(a);if(r.model_export){const model=document.createElement("a");model.textContent="Download GLB";model.href=r.model_export.replace("target/content-index/","");model.download="";actions.append(model)}d.append(actions);const state=document.createElement("div");state.className="status green";state.textContent=r.preview.representation||"Preview prepared";d.append(state);if(r.preview.dependencies&&r.preview.dependencies.length){const title=document.createElement("div");title.className="section";title.textContent="Resource dependencies";d.append(title);for(const dep of r.preview.dependencies){const line=document.createElement("div");line.className="meta";line.style.cssText="white-space:normal;overflow-wrap:anywhere;margin:4px 0";line.textContent=`${dep.kind} · ${dep.path}`;d.append(line)}}}else{const state=document.createElement("div");state.className="status";state.textContent="No offline preview has been prepared for this resource.";d.append(state);if(r.path_kind==="logical"){const cmd=document.createElement("code");cmd.style.cssText="display:block;margin-top:10px;color:#9eb2c1;overflow-wrap:anywhere";cmd.textContent=`python tools/asset_workbench/prepare_preview.py ${r.path}`;d.append(cmd)}}}
document.getElementById("search").addEventListener("input",()=>{page=0;applyFilters()});document.getElementById("preview").onchange=applyFilters;document.getElementById("spawn").onchange=applyFilters;document.getElementById("previous").onclick=()=>{page=Math.max(0,page-1);renderRows()};document.getElementById("next").onclick=()=>{page++;renderRows()};applyFilters();
</script></html>"""


if __name__ == "__main__":
    raise SystemExit(main())

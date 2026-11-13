//! The stylesheet. Colours come from the theme as CSS custom properties.

/// `/style.css`.
pub const CSS: &str = r#"
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--text);font:14px/1.45 ui-sans-serif,system-ui,-apple-system,"Segoe UI",sans-serif}
a{color:var(--accent);text-decoration:none}a:hover{text-decoration:underline}
header.top{background:var(--surface);border-bottom:1px solid var(--border);padding:0 16px;display:flex;gap:4px;align-items:center;position:sticky;top:0;z-index:5}
header.top .brand{font-weight:700;margin-right:12px;color:var(--accent)}
nav a{display:inline-block;padding:10px 12px;color:var(--text2);border-bottom:2px solid transparent}
nav a.on{color:var(--text);border-bottom-color:var(--accent)}
nav a.bell{margin-left:auto}
main{max-width:1180px;margin:0 auto;padding:18px 16px 60px}
h1{font-size:20px;margin:0 0 4px}h2{font-size:15px;margin:18px 0 8px;color:var(--text2)}
.dim,.meta{color:var(--muted)}.small{font-size:12px}
table{width:100%;border-collapse:collapse}
td,th{padding:7px 8px;border-bottom:1px solid var(--border);text-align:left;vertical-align:top}
tr:hover td{background:var(--sel)}
.mark{width:1.4em;text-align:center}.ok{color:var(--ok)}.err{color:var(--err)}.warn{color:var(--warn)}.mu{color:var(--muted)}.acc{color:var(--accent)}
.badge{display:inline-block;padding:1px 8px;border-radius:999px;border:1px solid var(--border);font-size:12px;color:var(--text2)}
.badge.open{color:var(--ok);border-color:var(--ok)}.badge.merged{color:var(--accent);border-color:var(--accent)}.badge.closed{color:var(--err);border-color:var(--err)}
.panel{background:var(--surface);border:1px solid var(--border);border-radius:8px;padding:12px 14px;margin:12px 0}
.bar{display:flex;gap:8px;flex-wrap:wrap;align-items:center;margin:10px 0}
button,.btn{font:inherit;background:var(--surface);color:var(--text);border:1px solid var(--border);border-radius:6px;padding:5px 12px;cursor:pointer}
button:hover{border-color:var(--accent)}button.primary{background:var(--accent);color:var(--bg);border-color:var(--accent)}button.danger{color:var(--err)}
textarea{width:100%;min-height:80px;background:var(--bg);color:var(--text);border:1px solid var(--border);border-radius:6px;padding:8px;font:inherit}
pre{margin:0;white-space:pre-wrap;word-break:break-word;font:12px/1.5 ui-monospace,Menlo,Consolas,monospace}
.diff{background:var(--surface);border:1px solid var(--border);border-radius:6px;overflow:auto}
.diff div{padding:0 10px;white-space:pre;font:12px/1.5 ui-monospace,Menlo,Consolas,monospace}
.diff .add{color:var(--add)}.diff .del{color:var(--del)}.diff .hunk{color:var(--hunk)}.diff .ctx{color:var(--muted)}
.log{background:var(--surface);border:1px solid var(--border);border-radius:6px;padding:8px 0;overflow:auto}
.log .ln{display:block;padding:0 10px}.log .ln a{color:var(--muted);display:inline-block;min-width:4ch;margin-right:1.2ch;text-align:right}
.log .ln.err{color:var(--err)}.log .ln.warn{color:var(--warn)}.log .ln.cmd{color:var(--accent)}.log .ln:target{background:var(--sel)}
.log details{margin-left:1.5ch}.log summary{cursor:pointer;color:var(--text);font-weight:600}
.tabs a{margin-right:14px;color:var(--text2)}.tabs a.on{color:var(--text);font-weight:600}
.comment{border-left:3px solid var(--border);padding:4px 12px;margin:10px 0}.comment b{color:var(--accent)}
img.shot{max-width:100%;border:1px solid var(--border);border-radius:6px;margin:6px 0}
.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(150px,1fr));gap:10px}
.stat{background:var(--surface);border:1px solid var(--border);border-radius:8px;padding:10px 12px}.stat b{display:block;font-size:22px}
svg .lbl{font:10px sans-serif;fill:var(--muted)}svg.heat{max-width:100%;height:auto}
.note{padding:6px 10px;border-radius:6px;background:var(--sel);margin:8px 0}
"#;

/// The script every page loads: sends an action as JSON and reloads.
pub const SCRIPT: &str = r#"
async function act(p){
  const r=await fetch('/api/action',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(Object.assign({token:document.querySelector('meta[name=gk-token]').content},p))});
  const t=await r.text();
  if(!r.ok){alert(t);return;}
  location.reload();
}
function actBtn(b){
  const p=JSON.parse(b.dataset.act);
  if(b.dataset.from){p.body=document.getElementById(b.dataset.from).value;}
  if(b.dataset.confirm&&!confirm(b.dataset.confirm))return;
  act(p);
}
"#;

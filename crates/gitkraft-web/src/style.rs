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
svg rect.day{cursor:pointer;outline:none}svg rect.day:hover,svg rect.day:focus{stroke:var(--text);stroke-width:1.5}svg rect.day.sel{stroke:var(--accent);stroke-width:2}
#day h3{margin:10px 0 4px;font-size:14px}#day ul{margin:0;padding-left:18px}#day li{margin:2px 0}#day .repo{color:var(--muted)}
svg .lbl{font:10px sans-serif;fill:var(--muted)}svg.heat{max-width:100%;height:auto}
#gk-pal{position:fixed;inset:0;background:rgba(0,0,0,.45);display:flex;justify-content:center;align-items:flex-start;padding-top:12vh;z-index:50}
#gk-pal[hidden]{display:none}
#gk-pal .box{width:min(640px,92vw);background:var(--surface);border:1px solid var(--border);border-radius:10px;overflow:hidden;box-shadow:0 12px 40px rgba(0,0,0,.4)}
#gk-pal input{width:100%;border:0;border-bottom:1px solid var(--border);background:var(--bg);color:var(--text);padding:12px 14px;font:inherit;outline:none}
#gk-pal ul{list-style:none;margin:0;padding:4px 0;max-height:50vh;overflow:auto}
#gk-pal li{padding:6px 14px;cursor:pointer;display:flex;justify-content:space-between;gap:12px}
#gk-pal li.on{background:var(--sel)}#gk-pal li small{color:var(--muted)}#gk-pal li.danger{color:var(--err)}
#gk-pal .none{padding:10px 14px;color:var(--muted)}
button.pal-hint{margin-left:8px;font-size:12px;color:var(--muted)}
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
/* Command palette: every link of the navigation and every action button of the page, by fuzzy name. */
function gkEntries(){
  const out=[],seen=new Set();
  const add=(label,hint,run,danger)=>{const k=label+'\u0000'+hint;if(label&&!seen.has(k)){seen.add(k);out.push({label,hint,run,danger});}};
  document.querySelectorAll('header.top nav a').forEach(a=>add('Go to '+a.textContent.trim(),'',()=>{location.href=a.href;}));
  document.querySelectorAll('nav.tabs a').forEach(a=>add('View: '+a.textContent.trim(),'',()=>{location.href=a.href;}));
  document.querySelectorAll('button[data-act]').forEach(b=>add(b.textContent.trim(),b.dataset.confirm?'asks first':'',()=>b.click(),b.classList.contains('danger')));
  add('Refresh','',()=>location.reload());
  return out;
}
function gkScore(q,s){
  q=q.toLowerCase();s=s.toLowerCase();
  if(!q)return 1;
  const at=s.indexOf(q);
  if(at>=0)return 1000-at;
  let i=0,score=0,last=-2;
  for(let j=0;j<s.length&&i<q.length;j++){
    if(s[j]===q[i]){score+=(j===last+1)?3:1;last=j;i++;}
  }
  return i===q.length?score:0;
}
function gkPalette(){
  let pal=document.getElementById('gk-pal');
  if(!pal){
    pal=document.createElement('div');pal.id='gk-pal';
    pal.innerHTML='<div class="box"><input placeholder="Type a command…" aria-label="Command palette"><ul role="listbox"></ul></div>';
    pal.addEventListener('mousedown',e=>{if(e.target===pal)pal.hidden=true;});
    document.body.appendChild(pal);
  }
  const input=pal.querySelector('input'),list=pal.querySelector('ul');
  let all=gkEntries(),shown=[],sel=0;
  const draw=()=>{
    shown=all.map(e=>[gkScore(input.value,e.label),e]).filter(x=>x[0]>0).sort((a,b)=>b[0]-a[0]).map(x=>x[1]);
    sel=Math.min(sel,Math.max(shown.length-1,0));
    list.innerHTML='';
    if(!shown.length){list.innerHTML='<div class="none">nothing matches</div>';return;}
    shown.forEach((e,i)=>{
      const li=document.createElement('li');
      if(i===sel)li.className='on';
      if(e.danger)li.classList.add('danger');
      li.setAttribute('role','option');
      const l=document.createElement('span');l.textContent=e.label;li.appendChild(l);
      if(e.hint){const h=document.createElement('small');h.textContent=e.hint;li.appendChild(h);}
      li.onmousedown=ev=>{ev.preventDefault();run(i);};
      list.appendChild(li);
    });
    const on=list.querySelector('.on');if(on)on.scrollIntoView({block:'nearest'});
  };
  const close=()=>{pal.hidden=true;input.onkeydown=null;input.oninput=null;};
  const run=i=>{const e=shown[i];close();if(e)e.run();};
  input.oninput=()=>{sel=0;draw();};
  input.onkeydown=e=>{
    if(e.key==='Escape'){close();}
    else if(e.key==='ArrowDown'||(e.ctrlKey&&e.key==='n')){e.preventDefault();sel=Math.min(sel+1,shown.length-1);draw();}
    else if(e.key==='ArrowUp'||(e.ctrlKey&&e.key==='p')){e.preventDefault();sel=Math.max(sel-1,0);draw();}
    else if(e.key==='Enter'){e.preventDefault();run(sel);}
  };
  input.value='';sel=0;pal.hidden=false;draw();input.focus();
}
document.addEventListener('keydown',e=>{
  const t=e.target,typing=t&&(t.tagName==='INPUT'||t.tagName==='TEXTAREA'||t.tagName==='SELECT'||t.isContentEditable);
  if((e.metaKey||e.ctrlKey)&&e.key.toLowerCase()==='k'){e.preventDefault();gkPalette();}
  else if(!typing&&!e.metaKey&&!e.ctrlKey&&!e.altKey&&(e.key===':'||e.key==='/')){e.preventDefault();gkPalette();}
});
document.addEventListener('DOMContentLoaded',()=>{
  const top=document.querySelector('header.top');
  if(top){const b=document.createElement('button');b.className='pal-hint';b.title='Command palette (Ctrl/Cmd+K or :)';b.textContent='⌘K';b.onclick=gkPalette;top.appendChild(b);}
});
/* Calendar: a day opens what you did that day, like GitHub's panel; arrows move the cursor. */
function gkDay(rect){
  const panel=document.getElementById('day');if(!panel||!rect)return;
  document.querySelectorAll('svg.heat rect.sel').forEach(r=>r.classList.remove('sel'));
  rect.classList.add('sel');
  panel.hidden=false;panel.textContent='Loading '+rect.dataset.date+'…';
  fetch('/api/day?date='+encodeURIComponent(rect.dataset.date)).then(r=>r.ok?r.json():r.text().then(t=>{throw new Error(t);})).then(d=>{
    if(!rect.classList.contains('sel'))return;
    panel.textContent='';
    const h=document.createElement('b');h.textContent=d.headline;panel.appendChild(h);
    d.errors.forEach(e=>{const n=document.createElement('div');n.className='note warn';n.textContent=e;panel.appendChild(n);});
    d.groups.forEach(g=>{
      const t=document.createElement('h3');t.textContent=g.heading+' · '+g.count;panel.appendChild(t);
      const ul=document.createElement('ul');
      g.items.forEach(i=>{
        const li=document.createElement('li');
        const label=(i.number?'#'+i.number+' ':'')+i.title;
        if(/^https?:\/\//.test(i.url)){const a=document.createElement('a');a.href=i.url;a.textContent=label;a.target='_blank';a.rel='noopener';li.appendChild(a);}
        else li.appendChild(document.createTextNode(label));
        const r=document.createElement('span');r.className='repo';r.textContent=' · '+i.repo;li.appendChild(r);
        ul.appendChild(li);
      });
      panel.appendChild(ul);
    });
    if(!d.groups.length&&!d.errors.length){const n=document.createElement('div');n.className='dim';n.textContent='Nothing to show for this day.';panel.appendChild(n);}
  }).catch(e=>{panel.textContent='';const n=document.createElement('div');n.className='note err';n.textContent=String(e.message||e);panel.appendChild(n);});
}
function gkDayStep(rect,dx,dy){
  const d=new Date(rect.dataset.date+'T00:00:00Z');d.setUTCDate(d.getUTCDate()+dx*7+dy);
  const next=document.querySelector('svg.heat rect.day[data-date="'+d.toISOString().slice(0,10)+'"]');
  if(next)next.focus();
}
document.addEventListener('click',e=>{const r=e.target.closest&&e.target.closest('svg.heat rect.day');if(r)gkDay(r);});
document.addEventListener('keydown',e=>{
  const r=e.target.closest&&e.target.closest('svg.heat rect.day');if(!r)return;
  const step={ArrowLeft:[-1,0],ArrowRight:[1,0],ArrowUp:[0,-1],ArrowDown:[0,1]}[e.key];
  if(step){e.preventDefault();gkDayStep(r,step[0],step[1]);}
  else if(e.key==='Enter'||e.key===' '){e.preventDefault();gkDay(r);}
});
function actBtn(b){
  const p=JSON.parse(b.dataset.act);
  if(b.dataset.from){p.body=document.getElementById(b.dataset.from).value;}
  if(b.dataset.method){p.method=document.getElementById(b.dataset.method).value;}
  if(b.dataset.confirm&&!confirm(b.dataset.confirm))return;
  act(p);
}
"#;

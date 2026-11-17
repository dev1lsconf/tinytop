const state = { range: 24, refreshMs: 30000, timer: null };
    function $(id) { return document.getElementById(id); }
    function fmtPct(v) { const n = Number(v || 0); return n.toFixed(1) + '%'; }
    function fmtMB(v) { const n = Number(v || 0); if (n >= 1024) return (n/1024).toFixed(2) + ' GB'; return n.toFixed(0) + ' MB'; }
    function fmtBytes(n) { const v = Number(n || 0); if (v < 1024) return v.toFixed(0) + ' B'; const kb=v/1024; if(kb<1024) return kb.toFixed(1)+' KB'; const mb=kb/1024; if(mb<1024) return mb.toFixed(1)+' MB'; const gb=mb/1024; if(gb<1024) return gb.toFixed(2)+' GB'; return (gb/1024).toFixed(2)+' TB'; }
    function fmtRate(n) { return fmtBytes(n) + '/s'; }
    function fmtTs(ts) { if (!ts) return '—'; const d = new Date(ts); return isNaN(d) ? String(ts) : d.toLocaleString(); }
    function fmtUptime(sec){ if(sec==null) return '--'; sec=Number(sec); const d=Math.floor(sec/86400), h=Math.floor((sec%86400)/3600), m=Math.floor((sec%3600)/60); if(d>0) return d+'d '+h+'h'; if(h>0) return h+'h '+m+'m'; return m+'m'; }
    function esc(s) { return String(s ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c])); }

    function setHealth(kind, text) { const b = $('healthBadge'); b.className = 'badge ' + (kind || 'warn'); $('healthText').textContent = text; }
    function setError(msg) { const b = $('errorBanner'); if (msg) { b.textContent = msg; b.classList.add('show'); } else { b.textContent = ''; b.classList.remove('show'); } }

    const pctTick = (v) => (v == null ? '' : Number(v).toFixed(0) + '%');
    const baseOpts = { responsive:true, maintainAspectRatio:false, animation:false, interaction:{mode:'index',intersect:false}, plugins:{legend:{display:false}, tooltip:{enabled:true,displayColors:false}}, scales:{ x:{grid:{color:'rgba(255,255,255,0.05)'},ticks:{color:'#71717a',maxTicksLimit:8}}, y:{beginAtZero:true,grid:{color:'rgba(255,255,255,0.05)'},ticks:{color:'#71717a'}} } };
    const pctOpts = { ...baseOpts, plugins:{legend:{display:false}, tooltip:{enabled:true,displayColors:false, callbacks:{ label:(ctx)=> (ctx.dataset.label||'') + ': ' + Number(ctx.parsed.y||0).toFixed(1) + '%' }}}, scales:{ ...baseOpts.scales, y:{...baseOpts.scales.y, min:0, max:100, ticks:{color:'#71717a', callback:pctTick}} } };
    const rateOpts = { ...baseOpts, plugins:{legend:{display:true,labels:{color:'#a1a1aa',boxWidth:10}}, tooltip:{enabled:true, callbacks:{ label:(ctx)=> (ctx.dataset.label||'') + ': ' + fmtRate(ctx.parsed.y||0) }}}, scales:{ ...baseOpts.scales, y:{...baseOpts.scales.y, ticks:{color:'#71717a', callback:(v)=>fmtRate(v)}} } };
    const loadOpts = { ...baseOpts, plugins:{legend:{display:true,labels:{color:'#a1a1aa',boxWidth:10}}, tooltip:{enabled:true, callbacks:{ label:(ctx)=> (ctx.dataset.label||'') + ': ' + Number(ctx.parsed.y||0).toFixed(2) }}} };
    const cpuChart = new Chart($('cpuChart'), { type:'line', data:{labels:[], datasets:[{label:'CPU', data:[], borderColor:'#3b82f6', backgroundColor:'rgba(59,130,246,0.18)', fill:true, borderWidth:2, pointRadius:0, tension:0.3}]}, options:pctOpts });
    const memChart = new Chart($('memoryChart'), { type:'line', data:{labels:[], datasets:[{label:'Memoria', data:[], borderColor:'#22c55e', backgroundColor:'rgba(34,197,94,0.18)', fill:true, borderWidth:2, pointRadius:0, tension:0.3}]}, options:pctOpts });
    const netChart = new Chart($('netChart'), { type:'line', data:{labels:[], datasets:[{label:'RX', data:[], borderColor:'#38bdf8', backgroundColor:'rgba(56,189,248,0.12)', fill:true, borderWidth:2, pointRadius:0, tension:0.3},{label:'TX', data:[], borderColor:'#f472b6', backgroundColor:'rgba(244,114,182,0.10)', fill:true, borderWidth:2, pointRadius:0, tension:0.3}]}, options:rateOpts });
    const loadChart = new Chart($('loadChart'), { type:'line', data:{labels:[], datasets:[{label:'1m', data:[], borderColor:'#f59e0b', borderWidth:2, pointRadius:0, tension:0.3, fill:false},{label:'5m', data:[], borderColor:'#a78bfa', borderWidth:2, pointRadius:0, tension:0.3, fill:false},{label:'15m', data:[], borderColor:'#34d399', borderWidth:2, pointRadius:0, tension:0.3, fill:false}]}, options:loadOpts });

    function renderSystem(system) {
      if (!system) { $('cpuValue').textContent='--%'; $('cpuBar').style.width='0%'; $('memoryValue').textContent='--'; $('memoryBar').style.width='0%'; $('netRateValue').textContent='--'; $('netTotals').textContent='RX total: -- · TX total: --'; $('kvLoad').textContent='--'; $('kvUptime').textContent='--'; $('kvProcs').textContent='--'; return; }
      const cpu=Number(system.cpu_percent||0), mup=Number(system.mem_used_percent||0);
      $('cpuValue').textContent = fmtPct(cpu); $('cpuBar').style.width = Math.min(Math.max(cpu,0),100)+'%'; $('cpuMeta').textContent = cpu>80?'Uso alto':cpu>50?'Uso moderado':'Uso normal';
      $('memoryValue').textContent = fmtPct(mup); $('memoryBar').style.width = Math.min(Math.max(mup,0),100)+'%'; $('memoryMeta').textContent = fmtMB(system.mem_used_mb)+' / '+fmtMB(system.mem_total_mb);
      $('netRateValue').textContent = '↓ '+fmtRate(system.net_rx_bytes_per_sec)+' · ↑ '+fmtRate(system.net_tx_bytes_per_sec);
      $('netTotals').textContent = 'RX total: '+fmtBytes(system.net_rx_bytes)+' · TX total: '+fmtBytes(system.net_tx_bytes);
      const l1=system.load1, l5=system.load5, l15=system.load15; $('kvLoad').textContent = (l1!=null?Number(l1).toFixed(2):'--')+' / '+(l5!=null?Number(l5).toFixed(2):'--')+' / '+(l15!=null?Number(l15).toFixed(2):'--');
      $('kvUptime').textContent = fmtUptime(system.uptime_seconds);
      try { const tp = system.top_processes ? JSON.parse(system.top_processes) : []; $('kvProcs').textContent = (Array.isArray(tp)&&tp.length) ? tp.slice(0,3).map(p=> (p.name||'?')+' '+Number(p.cpu||0).toFixed(0)+'%').join(', ') : '--'; } catch { $('kvProcs').textContent = '--'; }
    }

    function statusPill(status) { const s=String(status||'unknown').toLowerCase(); const cls=(s==='up'||s==='ok'||s==='resolved'||s==='info')?'up':(s==='down'||s==='error'||s==='firing'||s==='critical')?s:(s==='degraded'||s==='starting'||s==='warn'||s==='warning')?s:'unknown'; return '<span class="pill '+cls+'">'+esc(s.toUpperCase())+'</span>'; }

    function renderTargets(targets) {
      const tbody=$('targetsTable'); const list=Array.isArray(targets)?targets:[]; $('targetsCount').textContent=list.length+' target'+(list.length===1?'':'s'); tbody.textContent='';
      if(!list.length){ const tr=document.createElement('tr'); const td=document.createElement('td'); td.colSpan=7; td.className='muted'; td.textContent='No hay targets configurados'; tr.appendChild(td); tbody.appendChild(tr); return; }
      for(const t of list){
        const tr=document.createElement('tr');
        const reqtls = (t.cert_days_left!=null) ? ('cert '+t.cert_days_left+'d') : (t.http_requests!=null?String(t.http_requests):'—');
        const c1=document.createElement('td'); const d=document.createElement('div'); d.className='row-title'; d.textContent=t.name||'unknown'; const u=document.createElement('div'); u.className='mono'; u.textContent=t.url||''; c1.appendChild(d); c1.appendChild(u);
        const c2=document.createElement('td'); c2.textContent=(t.check_type||t.kind||'http');
        const c3=document.createElement('td'); c3.innerHTML=statusPill(t.status);
        const c4=document.createElement('td'); c4.textContent=(t.latency_ms!=null)?Number(t.latency_ms).toFixed(1)+' ms':'—';
        const c5=document.createElement('td'); c5.textContent=reqtls;
        const c6=document.createElement('td'); c6.className='muted'; c6.textContent=t.detail||'—';
        const c7=document.createElement('td'); c7.className='muted'; c7.textContent=fmtTs(t.timestamp);
        [c1,c2,c3,c4,c5,c6,c7].forEach(c=>tr.appendChild(c)); tbody.appendChild(tr);
      }
    }

    function renderAlerts(alerts){ const tb=$('alertsTable'); const list=Array.isArray(alerts)?alerts:[]; const firing=list.filter(a=>a.status==='firing'); $('kvAlerts').textContent=firing.length+' firing / '+list.length; $('alertsMeta').textContent=list.length+' total'; tb.textContent=''; if(!list.length){ const tr=document.createElement('tr'); const td=document.createElement('td'); td.colSpan=5; td.className='muted'; td.textContent='Sin alertas'; tr.appendChild(td); tb.appendChild(tr); return; } for(const a of list.slice(0,50)){ const tr=document.createElement('tr'); const cells=[statusPill(a.status), statusPill(a.severity), esc(a.source||''), esc(a.message||''), fmtTs(a.created_at)]; const tds=cells.map((h,i)=>{const td=document.createElement('td'); if(i<2) td.innerHTML=h; else td.textContent=String(h).replace(/&amp;|&lt;|&gt;|&quot;|&#39;/g,''); return td;}); tds[2].textContent=a.source||''; tds[3].textContent=a.message||''; tds[4].textContent=fmtTs(a.created_at); tds.forEach(td=>tr.appendChild(td)); tb.appendChild(tr);} }
    function renderEvents(events){ const tb=$('eventsTable'); const list=Array.isArray(events)?events:[]; $('eventsMeta').textContent=list.length+' recientes'; tb.textContent=''; if(!list.length){ const tr=document.createElement('tr'); const td=document.createElement('td'); td.colSpan=5; td.className='muted'; td.textContent='Sin eventos'; tr.appendChild(td); tb.appendChild(tr); return; } for(const e of list.slice(0,50)){ const tr=document.createElement('tr'); const c1=document.createElement('td'); c1.innerHTML=statusPill(e.level||'info'); const c2=document.createElement('td'); c2.textContent=e.kind||''; const c3=document.createElement('td'); c3.textContent=e.source||''; const c4=document.createElement('td'); c4.textContent=e.message||''; const c5=document.createElement('td'); c5.textContent=fmtTs(e.timestamp); [c1,c2,c3,c4,c5].forEach(c=>tr.appendChild(c)); tb.appendChild(tr);} }

    async function loadLatest(){ const res=await fetchWithAuth('/api/metrics/latest'); if(!res.ok){ const e=new Error('latest: HTTP '+res.status); e.status=res.status; throw e; } const data=await res.json(); renderSystem(data.system); renderTargets(data.targets||[]); return data; }
    async function loadHistory(){
      const url='/api/metrics/history?target=system&range='+encodeURIComponent(state.range);
      const res=await fetchWithAuth(url); if(!res.ok){ const e=new Error('history: HTTP '+res.status); e.status=res.status; throw e; }
      const data=await res.json(); const points=Array.isArray(data.points)?data.points:[];
      const has=points.length>0;
      const down = !!data.downsampled;
      const bucket = data.bucket_seconds;
      const aggLabel = down ? (' · rollup ' + (bucket ? (bucket>=3600?(bucket/3600)+'h':(bucket/60)+'m') : '')) : ' · raw';
      const hasNonNull = (arr)=>arr.some(v=>v!=null);
      [['cpuChartEmpty','cpuChartMeta'],['memoryChartEmpty','memoryChartMeta'],['netChartEmpty','netChartMeta']].forEach(([e,m])=>{ if($(e)) $(e).classList.toggle('show',!has); if($(m)) $(m).textContent=has?(points.length+' puntos'+aggLabel):'0 puntos'; });
      if($('loadChartEmpty')) $('loadChartEmpty').textContent = down ? 'No disponible en rollup' : 'Sin datos aún';
      const labels=points.map(p=>{const d=new Date(p.timestamp); return isNaN(d)?'':d.toLocaleTimeString([], {hour:'2-digit',minute:'2-digit'});});
      cpuChart.data.labels=labels; cpuChart.data.datasets[0].data=points.map(p=>Number(p.cpu_percent||0));
      memChart.data.labels=labels; memChart.data.datasets[0].data=points.map(p=>Number(p.mem_used_percent||0));
      netChart.data.labels=labels; netChart.data.datasets[0].data=points.map(p=>p.net_rx_bytes_per_sec==null?null:Number(p.net_rx_bytes_per_sec)); netChart.data.datasets[1].data=points.map(p=>p.net_tx_bytes_per_sec==null?null:Number(p.net_tx_bytes_per_sec));
      const l1=points.map(p=>p.load1==null?null:Number(p.load1)); const l5=points.map(p=>p.load5==null?null:Number(p.load5)); const l15=points.map(p=>p.load15==null?null:Number(p.load15));
      loadChart.data.labels=labels; loadChart.data.datasets[0].data=l1; loadChart.data.datasets[1].data=l5; loadChart.data.datasets[2].data=l15;
      const loadHas = has && hasNonNull(l1.concat(l5,l15));
      if($('loadChartEmpty')) $('loadChartEmpty').classList.toggle('show', !loadHas);
      if($('loadChartMeta')) $('loadChartMeta').textContent = has ? (points.length+' puntos'+aggLabel + (loadHas?'':' · n/d')) : '0 puntos';
      [cpuChart,memChart,netChart,loadChart].forEach(c=>c.update());
      return data;
    }
    async function loadHealth(){ try{ const res=await fetch('/api/health',{cache:'no-store'}); const data=await res.json().catch(()=>({})); $('version').textContent='v'+(data.version||'0.1.0'); if(res.ok&&data.ready) setHealth('ok','En línea'); else if(res.ok) setHealth('warn',data.status||'Iniciando'); else setHealth('err',data.status||'Degradado'); }catch{ setHealth('err','Sin conexión'); } }
    async function loadAlerts(){ try{ const res=await fetchWithAuth('/api/alerts?limit=50'); if(!res.ok) throw 0; const d=await res.json(); renderAlerts(d.alerts||[]); }catch{ renderAlerts([]); $('kvAlerts').textContent='--'; } }
    async function loadEvents(){ try{ const res=await fetchWithAuth('/api/events?limit=50'); if(!res.ok) throw 0; const d=await res.json(); renderEvents(d.events||[]); }catch{ renderEvents([]); } }
    function updateExportLinks(){ const r=state.range; $('exportJson').href='/api/export?target=system&format=json&range='+r; $('exportCsv').href='/api/export?target=system&format=csv&range='+r; }

    function getAuthToken(){ return sessionStorage.getItem('tinytop_token') || ''; }
    function setAuthToken(t){ if(t) sessionStorage.setItem('tinytop_token', t); else sessionStorage.removeItem('tinytop_token'); }

    async function fetchWithAuth(url, opts){
      const base = Object.assign({cache:'no-store'}, opts || {});
      base.headers = Object.assign({}, base.headers || {});
      const t = getAuthToken();
      if(t && !base.headers['Authorization'] && !base.headers['authorization']) base.headers['Authorization'] = 'Bearer ' + t;
      let res = await fetch(url, base);
      if(res.status === 401 && typeof window.prompt === 'function'){
        const entered = window.prompt('Autenticación requerida. Introduce el token (Bearer/X-API-Key). Cancelar para omitir:');
        if(entered && entered.trim()){
          setAuthToken(entered.trim());
          const retryHeaders = Object.assign({}, base.headers, {'Authorization':'Bearer '+entered.trim()});
          res = await fetch(url, Object.assign({}, base, {headers: retryHeaders}));
        }
      }
      return res;
    }

    async function refresh(){ try{ updateExportLinks(); await Promise.all([loadLatest(), loadHistory(), loadHealth(), loadAlerts(), loadEvents()]); setError(''); }catch(e){ console.error(e); if(e && e.status===401){ setError('No autorizado: pulsa "Token" para introducir credenciales.'); setHealth('err','No autorizado'); } else { setError('No se pudieron cargar los datos: '+(e&&e.message?e.message:'error')); setHealth('err','Error'); } } finally { $('lastUpdate').textContent=new Date().toLocaleTimeString(); } }
    function applyAutoRefresh(){ if(state.timer){ clearInterval(state.timer); state.timer=null; } if(state.refreshMs>0) state.timer=setInterval(refresh, state.refreshMs); }
    $('rangeSelect').addEventListener('change',(e)=>{ state.range=Number(e.target.value||24); refresh(); });
    $('refreshSelect').addEventListener('change',(e)=>{ state.refreshMs=Number(e.target.value||0)*1000; applyAutoRefresh(); });
    $('authBtn').addEventListener('click',()=>{
      const current = getAuthToken();
      const entered = window.prompt('Token de acceso (Bearer/X-API-Key). Se guarda solo en esta sesión. Deja vacío para borrar:', current);
      if(entered===null) return;
      const v = entered.trim();
      setAuthToken(v);
      if(v) { setError('Token guardado en esta sesión.'); } else { setError('Token eliminado.'); }
      refresh();
    });
    $('refreshBtn').addEventListener('click',()=>refresh());
    applyAutoRefresh(); refresh();

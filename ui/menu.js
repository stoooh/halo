// Menu radiale di Halo.
// - Dentro l'app Tauri: legge il file passato da riga di comando, esegue le azioni reali.
// - Aperto in un browser: modalità anteprima con sfondo e schede per provare i tipi di file.
(function () {
  'use strict';

  const IC = window.HALO_ICONS;
  const TYPES = window.HALO_TYPES;
  const slug = window.haloSlug;
  const TAURI = window.__TAURI__;
  const inApp = !!(TAURI && TAURI.core);
  const NS = 'http://www.w3.org/2000/svg';
  const MENU = 460, RING = 420;

  const st = {
    type: 0, level: null, focus: 0, hover: null, chosen: null, kb: false,
    busy: false, result: null, file: null, files: [], progress: null, query: '', searchSel: 0, supported: null, forms: null, panel: null,
    anim: 'rm-in', placed: false
  };
  const refs = { wedges: [], fills: [], focusPath: null };

  const $ = (id) => document.getElementById(id);
  const menu = $('menu'), ring = $('ring'), slot = $('center-slot'), toastEl = $('toast');

  // ---------- utilità DOM ----------
  function svg(w, h, attrs) {
    const s = document.createElementNS(NS, 'svg');
    s.setAttribute('width', w); s.setAttribute('height', h);
    s.setAttribute('aria-hidden', 'true');
    for (const k in attrs || {}) s.setAttribute(k, attrs[k]);
    return s;
  }
  function path(d, attrs) {
    const p = document.createElementNS(NS, 'path');
    p.setAttribute('d', d);
    for (const k in attrs || {}) p.setAttribute(k, attrs[k]);
    return p;
  }
  function icon(d, size, sw) {
    const s = svg(size || 24, size || 24, { viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor',
      'stroke-width': sw || 1.6, 'stroke-linecap': 'round', 'stroke-linejoin': 'round' });
    s.appendChild(path(d));
    return s;
  }
  function el(tag, cls, text) {
    const e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text != null) e.textContent = text;
    return e;
  }
  function say(msg) { $('status').textContent = msg; }

  // ---------- geometria degli spicchi ----------
  function wedge(i, n) {
    const R = 200, r = 92, C = 210, g = 10, c = 20;
    const step = (Math.PI * 2) / n;
    const mid = -Math.PI / 2 + i * step;
    const a0 = mid - step / 2, a1 = mid + step / 2;
    const dO = Math.asin(g / 2 / R), dI = Math.asin(g / 2 / r);
    const o0 = a0 + dO, o1 = a1 - dO, i0 = a0 + dI, i1 = a1 - dI;
    const P = (rho, th) => [C + rho * Math.cos(th), C + rho * Math.sin(th)];
    const L = (p, q, t) => [p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t];
    const f = (p) => p[0].toFixed(1) + ' ' + p[1].toFixed(1);
    const O0 = P(R, o0), O1 = P(R, o1), I0 = P(r, i0), I1 = P(r, i1);
    const k = c / Math.hypot(O0[0] - I0[0], O0[1] - I0[1]);
    const cO = c / R, cI = (c * 0.7) / r;
    const d = 'M ' + f(P(R, o0 + cO)) +
      ' A ' + R + ' ' + R + ' 0 0 1 ' + f(P(R, o1 - cO)) +
      ' Q ' + f(O1) + ' ' + f(L(O1, I1, k)) +
      ' L ' + f(L(I1, O1, k * 0.7)) +
      ' Q ' + f(I1) + ' ' + f(P(r, i1 - cI)) +
      ' A ' + r + ' ' + r + ' 0 0 0 ' + f(P(r, i0 + cI)) +
      ' Q ' + f(I0) + ' ' + f(L(I0, O0, k * 0.7)) +
      ' L ' + f(L(O0, I0, k)) +
      ' Q ' + f(O0) + ' ' + f(P(R, o0 + cO)) + ' Z';
    const lab = P((R + r) / 2 + 4, mid);
    return { d, lx: Math.round(lab[0]), ly: Math.round(lab[1]) };
  }

  // ---------- dati correnti ----------
  const T = () => TYPES[st.type];
  function items() {
    const t = T();
    if (st.level == null) return t.acts.map((a) => ({ label: a[0], icon: a[1], n: a[2].length }));
    return t.acts[st.level][2].map((s) => ({ label: s[0], icon: s[1] }));
  }
  function keyFor(i) {
    const t = T(), a = t.acts[st.level];
    if (isTogether(a)) return 'multi/insieme/' + slug(a[2][i][0]);
    return t.kind + '/' + slug(a[0]) + '/' + slug(a[2][i][0]);
  }
  // ---------- più file insieme ----------
  // «Insieme»: azioni che usano tutti i file scelti (ZIP, rinomina in serie, unisci, percorsi).
  const isTogether = (a) => !!(a && a[3] && a[3].together);
  const isMulti = () => st.files.length > 1;
  // Pannelli che hanno senso uguali per tutti i file; gli altri (editor, elenchi, orari…) vanno fatti uno alla volta.
  const BATCH_FORMS = new Set(['nuova-password', 'password-attuale', 'autore', 'titolo', 'artista', 'album', 'data', 'dimensioni',
    'velocita', 'parti', 'nome-serie', 'nome-serie-multi', 'ingrandisci', 'cerca', 'trova', 'sostituisci', 'filigrana', 'evidenzia', 'annota-pdf', 'annota-img', 'intervallo', 'estrai-pagine', 'sottotitoli',
    // Filigrana e firma uguali su tutti i file: l'immagine della firma si sceglie una volta.
    'firma', 'firma-doc']);
  function batchable(key) { return !st.forms || !st.forms.has(key) || BATCH_FORMS.has(st.forms.get(key)); }
  // Il menu per più file: stesse azioni del tipo (se i file sono dello stesso tipo) più «Insieme»;
  // con tipi diversi solo le azioni di «Insieme».
  function multiType(files) {
    const kinds = Array.from(new Set(files.map((f) => f.kind)));
    const same = kinds.length === 1 ? TYPES.find((t) => t.kind === kinds[0]) : null;
    const subs = [['ZIP', 'zip'], ['Per email', 'mail'], ['Rinomina in serie', 'rename'], ['Data di scatto', 'camera']];
    if (same && (same.kind === 'pdf' || same.kind === 'video')) subs.push(['Unisci', 'merge']);
    if (same && same.kind === 'image') subs.push(['Unisci in PDF', 'page']);
    subs.push(['Percorsi', 'copy']);
    const together = ['Insieme', 'pages', subs, { together: true }];
    if (same) return Object.assign({}, same, { multi: true, acts: [together].concat(same.acts.filter((a) => !isDirect(a) && !isCustom(a))) });
    return { kind: 'multi', tab: 'Più file', glyph: 'pages', band: '#6B6B6B', multi: true, mixed: true, acts: [together] };
  }
  // «Le mie app»: azione aggiunta dalle scorciatoie, sempre disponibile.
  const isCustom = (a) => !!(a && a[3] && a[3].custom);
  // Azione diretta (es. Traduci): nessun sottomenu, apre subito il pannello.
  const isDirect = (a) => !!(a && a[3] && a[3].direct);
  function supported(i) {
    if (st.level == null) {
      if (!st.supported || isCustom(T().acts[i]) || isTogether(T().acts[i])) return true;
      const t = T(), a = t.acts[i];
      if (isDirect(a)) return st.supported.has(t.kind + '/' + slug(a[0]));
      return a[2].some((_, j) => st.supported.has(t.kind + '/' + slug(a[0]) + '/' + slug(a[2][j][0])));
    }
    const a = T().acts[st.level];
    if (isTogether(a)) return true;
    if (isMulti() && !batchable(keyFor(i))) return false;
    return !st.supported || isCustom(a) || st.supported.has(keyFor(i));
  }
  function css(name) { return getComputedStyle(document.documentElement).getPropertyValue(name).trim(); }

  // ---------- disegno ----------
  function renderRing() {
    const list = items(), n = list.length, isSub = st.level != null;
    ring.textContent = '';
    ring.className = 'ring ' + (st.anim || '');
    ring.setAttribute('aria-label', isSub ? 'Opzioni di ' + T().acts[st.level][0] : 'Azioni per ' + fileName());
    refs.wedges = []; refs.fills = []; refs.edges = []; st.hover = null;

    // Un solo strato di vetro sfocato, fermo, ritagliato con la forma degli spicchi: tra uno spicchio
    // e l'altro resta trasparente. La sfocatura non sta sugli spicchi che si animano, perché
    // WebKitGTK durante l'animazione ne lasciava intravedere il bordo quadrato.
    const geo = list.map((_, i) => wedge(i, n));
    const blur = el('div', 'blur');
    const shapes = '<svg xmlns="http://www.w3.org/2000/svg" width="' + RING + '" height="' + RING + '">' +
      geo.map((g) => '<path d="' + g.d + '"/>').join('') + '</svg>';
    const mask = 'url("data:image/svg+xml,' + encodeURIComponent(shapes) + '")';
    blur.style.webkitMaskImage = blur.style.maskImage = mask;
    ring.appendChild(blur);

    list.forEach((it, i) => {
      const g = geo[i];
      const b = el('button', 'wedge');
      b.type = 'button';
      b.style.clipPath = "path('" + g.d + "')";
      const ok = supported(i);
      if (!ok) b.classList.add('todo');
      b.setAttribute('aria-label', isSub ? it.label + (ok ? '' : ' (non ancora disponibile)') : it.label + (it.n ? ', apre ' + it.n + ' opzioni' : ''));
      const lab = el('span', 'lab');
      lab.style.left = g.lx + 'px'; lab.style.top = g.ly + 'px';
      lab.appendChild(icon(IC[it.icon] || ''));
      lab.appendChild(el('span', 't', it.label));
      if (!isSub && it.n) {
        const dots = el('span', 'dots'); dots.setAttribute('aria-hidden', 'true');
        dots.append(el('i'), el('i'), el('i'));
        lab.appendChild(dots);
      } else if (!ok) {
        lab.appendChild(el('span', 'soon', isMulti() && st.supported && st.supported.has(keyFor(i)) ? 'un file alla volta' : 'presto'));
      }
      // Il colore dello spicchio è una forma SVG, non lo sfondo del pulsante: lo sfondo è un quadrato
      // e WebKitGTK, mentre anima l'ingrandimento, ne lasciava intravedere il bordo.
      const fs = svg(RING, RING, { class: 'wfill' });
      fs.appendChild(path(g.d));
      b.append(fs, lab);
      b.addEventListener('click', () => pick(i));
      // Acceso solo mentre il puntatore è sopra (con la tastiera conta lo spicchio selezionato).
      b.addEventListener('mouseenter', () => { st.hover = i; st.focus = i; paint(); });
      b.addEventListener('mouseleave', () => { if (st.hover === i) { st.hover = null; paint(); } });
      b.addEventListener('focus', () => setFocus(i));
      ring.appendChild(b);
      refs.wedges.push(b);
    });
    refs.fills = refs.wedges;

    const edges = svg(RING, RING);
    geo.forEach((g) => { const e = path(g.d, { fill: 'none', stroke: 'var(--edge)', 'stroke-width': 1 }); edges.appendChild(e); refs.edges.push(e); });
    refs.focusPath = path('', { fill: 'none', stroke: 'var(--focus)', 'stroke-width': 3, 'stroke-linejoin': 'round' });
    edges.appendChild(refs.focusPath);
    ring.appendChild(edges);
    refs.geo = geo;
    st.anim = '';
    paint();
    renderCenter();
  }

  // Lo spicchio sotto il puntatore (o selezionato con la tastiera) si ingrandisce verso l'esterno,
  // gli altri si rimpiccioliscono un poco; il bordo e il contorno di selezione li seguono.
  // Ingrandimento dello spicchio attivo: dalle impostazioni (Aspetto › Spicchio attivo).
  let SCALE_HOT = 1.07;
  const SCALE_REST = 1;
  function paint() {
    const isSub = st.level != null;
    const hotIdx = st.kb ? st.focus : st.hover;
    const anyHot = hotIdx != null && !(isSub && st.chosen === hotIdx) && !st.busy;
    refs.wedges.forEach((b, i) => {
      const hot = hotIdx === i;
      const chosen = isSub && st.chosen === i;
      b.classList.toggle('hot', hot && !chosen);
      b.classList.toggle('chosen', chosen);
      if (isSub) b.setAttribute('aria-pressed', chosen ? 'true' : 'false');
      const sc = !anyHot || (!hot && SCALE_REST === 1) || (hot && SCALE_HOT === 1) ? '' : hot ? 'scale(' + SCALE_HOT + ')' : 'scale(' + SCALE_REST + ')';
      b.style.transform = sc;
      if (refs.edges[i]) refs.edges[i].style.transform = sc;
    });
    const g = refs.geo && refs.geo[st.focus];
    refs.focusPath.setAttribute('d', st.kb && g ? g.d : '');
    refs.focusPath.style.transform = st.kb && anyHot ? 'scale(' + SCALE_HOT + ')' : '';
  }

  function fileName() { return isMulti() ? st.files.length + ' file' : st.file ? st.file.name : (inApp ? 'Nessun file' : T().demoFile); }

  function fileIcon() {
    const t = T();
    if (isMulti()) {
      // Più file: due fogli sovrapposti.
      const s = svg(52, 56, { viewBox: '0 0 52 56' });
      s.appendChild(path('M12 2.5h22l10 10V46a2.5 2.5 0 0 1-2.5 2.5h-29A2.5 2.5 0 0 1 10 46V5a2.5 2.5 0 0 1 2-2.5z', { fill: '#F3F3F3', stroke: 'rgba(0,0,0,0.14)' }));
      s.appendChild(path('M5 8.5h22l10 10V52a2.5 2.5 0 0 1-2.5 2.5h-29A2.5 2.5 0 0 1 3 52V11a2.5 2.5 0 0 1 2-2.5z', { fill: '#FFFFFF', stroke: 'rgba(0,0,0,0.14)' }));
      s.appendChild(path('M27 8.5V16a2.5 2.5 0 0 0 2.5 2.5H37', { fill: '#EDEDED', stroke: 'rgba(0,0,0,0.14)' }));
      return s;
    }
    if (t.folder) {
      const s = svg(56, 46, { viewBox: '0 0 56 46' });
      s.appendChild(path('M3 8a4 4 0 0 1 4-4h13l4 5h25a4 4 0 0 1 4 4v3H3z', { fill: '#3E9BE6' }));
      s.appendChild(path('M3 14a3 3 0 0 1 3-3h44a3 3 0 0 1 3 3v26a3 3 0 0 1-3 3H6a3 3 0 0 1-3-3z', { fill: '#6CBDF5' }));
      s.appendChild(path('M3 14a3 3 0 0 1 3-3h44a3 3 0 0 1 3 3v1H3z', { fill: '#9AD3FA' }));
      return s;
    }
    const ext = (fileName().split('.').pop() || '').toUpperCase().slice(0, 4);
    const wrap = el('span'); wrap.style.cssText = 'position:relative;display:block;width:42px;height:54px';
    const s = svg(42, 54, { viewBox: '0 0 42 54' });
    s.appendChild(path('M3 1.5h24l12 12V50a2.5 2.5 0 0 1-2.5 2.5h-33A2.5 2.5 0 0 1 1 50V4a2.5 2.5 0 0 1 2-2.5z', { fill: '#FFFFFF', stroke: 'rgba(0,0,0,0.14)' }));
    s.appendChild(path('M27 1.5V11a2.5 2.5 0 0 0 2.5 2.5H39', { fill: '#EDEDED', stroke: 'rgba(0,0,0,0.14)' }));
    const g = document.createElementNS(NS, 'g');
    g.setAttribute('transform', 'translate(11 14) scale(0.83)');
    g.appendChild(path(IC[t.glyph] || '', { fill: 'none', stroke: t.band, 'stroke-width': 1.9, 'stroke-linecap': 'round', 'stroke-linejoin': 'round' }));
    s.appendChild(g);
    const band = document.createElementNS(NS, 'rect');
    for (const [k, v] of Object.entries({ x: 1.5, y: 38, width: 37, height: 11, rx: 2, fill: t.band })) band.setAttribute(k, v);
    s.appendChild(band);
    wrap.appendChild(s);
    const label = el('span', null, ext);
    label.style.cssText = 'position:absolute;left:0;top:38px;width:40px;height:11px;display:flex;align-items:center;justify-content:center;color:#fff;font-size:7.5px;font-weight:700;letter-spacing:.05em';
    wrap.appendChild(label);
    return wrap;
  }

  function renderCenter() {
    slot.textContent = '';
    const isSub = st.level != null;
    let c;
    if (st.busy) {
      // Lavoro in corso: avanzamento (se il programma lo dice), tempo trascorso, Annulla e In sottofondo.
      c = el('div', 'center busy');
      const line = el('span', 'jobline', ''), btns = el('span', 'jobbtns');
      c.append(el('span', 'spinner'), el('span', 'ptitle', st.progress ? 'File ' + st.progress : 'Elaborazione…'), line, btns);
      if (st.job && inApp) {
        const stop = el('button', 'jobbtn', st.job.cancel ? 'Annullo…' : 'Annulla'), bg = el('button', 'jobbtn', 'In sottofondo');
        stop.type = bg.type = 'button';
        stop.disabled = st.job.cancel;
        stop.addEventListener('click', cancelJob);
        bg.addEventListener('click', hideJob);
        bg.hidden = true;
        btns.append(stop, bg);
      }
      updateJob(c);
    } else if (!isSub) {
      c = el('div', 'center ' + (st.animCenter || ''));
      c.append(fileIcon(), el('span', 'fname', fileName()));
      const li = inApp && lastIndex();
      if (li) {
        const a = T().acts[li[0]], b = el('button', 'again');
        b.type = 'button';
        b.title = 'Ripeti: ' + a[0] + ' › ' + a[2][li[1]][0];
        b.setAttribute('aria-label', b.title);
        b.append(icon('M3 12a9 9 0 1 0 3-6.7M3 4v5h5', 11, 2.4), document.createTextNode(a[2][li[1]][0]));
        b.addEventListener('click', () => goTo(li[0], li[1]));
        c.appendChild(b);
      }
    } else {
      const a = T().acts[st.level];
      c = el('button', 'center ' + (st.animCenter || ''));
      c.type = 'button';
      c.setAttribute('aria-label', 'Torna alle azioni (Esc). Ora in: ' + a[0]);
      c.addEventListener('click', back);
      const pi = el('span', 'picon'); pi.appendChild(icon(IC[a[1]], 22, 1.8));
      const chosen = el('span', 'pchosen', st.chosen != null ? a[2][st.chosen][0] : '');
      if (st.result) chosen.className = 'pchosen ' + (st.result.ok ? 'state-ok' : 'state-err');
      const bk = el('span', 'pback'); bk.appendChild(icon('M15 18l-6-6 6-6', 11, 2.6)); bk.appendChild(document.createTextNode('Indietro'));
      // File di tipi diversi: c'è solo «Insieme», quindi niente «Indietro».
      if (T().mixed) { c.disabled = true; bk.textContent = st.files.length + ' file'; }
      c.append(pi, el('span', 'ptitle', a[0]), chosen, bk);
      refs.back = c;
    }
    st.animCenter = '';
    slot.appendChild(c);
  }

  // `files`: i file creati; se ci sono compare «Mostra nella cartella».
  function toast(msg, kind, files) {
    toastEl.textContent = msg;
    toastEl.className = 'toast' + (kind ? ' state-' + kind : '');
    toastEl.hidden = !msg;
    if (msg && files && files.length && inApp) {
      const b = el('button', 'tact', 'Mostra nella cartella');
      b.type = 'button';
      b.addEventListener('click', async () => {
        try { await TAURI.core.invoke('show_in_folder', { paths: files }); } catch (e) { /* il file non c'è più */ }
        closeLater(300);
      });
      toastEl.append(el('br'), b);
    }
  }
  // Chiude il menu dopo un risultato; più tardi se c'è il pulsante da usare.
  function closeSoon() { closeLater(); }

  // ---------- «Ripeti»: l'ultima azione usata per questo tipo di file ----------
  function lastFor(kind) {
    try { return JSON.parse(localStorage.getItem('halo-ultima-' + kind) || 'null'); } catch (e) { return null; }
  }
  function saveLast(a, sub) {
    try { localStorage.setItem('halo-ultima-' + T().kind, JSON.stringify([a[0], sub[0]])); } catch (e) { /* senza memoria */ }
  }
  // Indici [azione, sottoazione] dell'ultima azione, se c'è ancora ed è disponibile.
  function lastIndex() {
    const l = !isMulti() && lastFor(T().kind);
    if (!l) return null;
    const ai = T().acts.findIndex((a) => a[0] === l[0] && !isCustom(a));
    const si = ai < 0 ? -1 : T().acts[ai][2].findIndex((s) => s[0] === l[1]);
    if (si < 0) return null;
    const key = T().kind + '/' + slug(l[0]) + '/' + slug(l[1]);
    return !st.supported || st.supported.has(key) ? [ai, si] : null;
  }
  function goTo(ai, si) {
    if (st.busy) return;
    closePanel(false); clearSearch();
    st.level = ai; st.focus = si; st.chosen = null; st.result = null;
    st.anim = 'rm-in'; st.animCenter = 'rm-in';
    toast('');
    renderRing();
    run(si);
  }

  // ---------- cerca scrivendo ----------
  const searchEl = el('div', 'search');
  searchEl.hidden = true;
  searchEl.setAttribute('role', 'listbox');
  searchEl.setAttribute('aria-label', 'Risultati della ricerca');
  menu.appendChild(searchEl);
  function results() {
    const q = slug(st.query.trim());
    if (!q) return [];
    const out = [];
    T().acts.forEach((a, ai) => {
      if (isDirect(a)) {
        if (slug(a[0]).includes(q) && (!st.supported || st.supported.has(T().kind + '/' + slug(a[0])))) out.push({ label: a[0], ai, si: -1 });
        return;
      }
      a[2].forEach((s, si) => {
        const label = a[0] + ' › ' + s[0];
        if (!slug(label).includes(q) && !slug(s[0]).includes(q)) return;
        const key = isTogether(a) ? 'multi/insieme/' + slug(s[0]) : T().kind + '/' + slug(a[0]) + '/' + slug(s[0]);
        const ok = isCustom(a) || isTogether(a) || ((!st.supported || st.supported.has(key)) && (!isMulti() || batchable(key)));
        if (ok) out.push({ label, ai, si });
      });
    });
    return out.slice(0, 6);
  }
  function renderSearch() {
    searchEl.textContent = '';
    if (!st.query) { searchEl.hidden = true; return; }
    toast('');
    const res = results();
    st.searchSel = Math.min(st.searchSel || 0, Math.max(0, res.length - 1));
    searchEl.appendChild(el('div', 'sq', 'Cerca: ' + st.query));
    if (!res.length) searchEl.appendChild(el('div', 'snone', 'Nessuna azione trovata.'));
    res.forEach((r, k) => {
      const b = el('button', 'sres', r.label);
      b.type = 'button';
      b.setAttribute('role', 'option');
      b.setAttribute('aria-selected', k === st.searchSel ? 'true' : 'false');
      b.addEventListener('click', () => chooseResult(r));
      searchEl.appendChild(b);
    });
    searchEl.hidden = false;
    say(res.length ? res.length + ' azioni: ' + res.map((r) => r.label).join(', ') + '. Frecce e Invio per scegliere.' : 'Nessuna azione trovata.');
  }
  function clearSearch() { st.query = ''; st.searchSel = 0; renderSearch(); }
  function chooseResult(r) {
    clearSearch();
    if (r.si < 0) {
      if (st.level != null && !T().mixed) { st.level = null; renderRing(); }
      pick(r.ai);
      return;
    }
    goTo(r.ai, r.si);
  }

  // ---------- interazione ----------
  function setFocus(i) {
    if (st.focus === i) return;
    st.focus = i;
    paint();
  }

  function focusWedge(i) {
    const b = refs.wedges[i];
    if (b) b.focus({ preventScroll: true });
  }

  function pick(i) {
    if (st.busy) return;
    if (st.query) clearSearch();
    if (st.level == null && isDirect(T().acts[i])) {
      if (!supported(i)) { toast('«' + T().acts[i][0] + '» non è ancora disponibile.'); return; }
      if (!inApp) { toast('Anteprima nel browser: qui le azioni non vengono eseguite.'); return; }
      closePanel(false);
      openTranslate(i);
      return;
    }
    if (st.level == null) {
      st.level = i; st.focus = 0; st.chosen = null; st.result = null;
      st.anim = 'rm-in'; st.animCenter = 'rm-in';
      toast('');
      renderRing();
      if (st.kb) focusWedge(0);
      const a = T().acts[i];
      say(a[0] + ': ' + a[2].map((s) => s[0]).join(', ') + '. Esc per tornare indietro.');
      return;
    }
    run(i);
  }

  function back() {
    if (st.level == null || st.busy || T().mixed) return;
    closePanel(false);
    const lv = st.level;
    st.level = null; st.focus = lv; st.chosen = null; st.result = null;
    st.anim = 'rm-back'; st.animCenter = 'rm-back';
    toast('');
    renderRing();
    if (st.kb) focusWedge(lv);
    say('Azioni principali.');
  }

  async function run(i) {
    const a = T().acts[st.level], label = a[0] + ' › ' + a[2][i][0];
    if (!supported(i)) {
      const b = refs.wedges[i];
      b.classList.remove('shake'); void b.offsetWidth; b.classList.add('shake');
      toast(isMulti() && st.supported && st.supported.has(keyFor(i)) ? '«' + label + '» funziona su un file alla volta.' : '«' + label + '» non è ancora disponibile.');
      return;
    }
    closePanel(false);
    const key = keyFor(i);
    if (inApp && st.forms && st.forms.has(key) && !isCustom(a)) { openPanel(i, key, st.forms.get(key)); return; }
    st.chosen = i; st.result = null;
    if (!inApp) {
      paint(); renderCenter();
      toast('Anteprima nel browser: qui le azioni non vengono eseguite.\nAvvia l\'app con Tauri per usarle davvero.');
      return;
    }
    await execAction(i, null);
  }

  // Esegue l'azione nel backend. Con `data` (campi del pannello) l'errore resta nel pannello.
  async function execAction(i, data) {
    const a = T().acts[st.level], label = a[0] + ' › ' + a[2][i][0];
    st.chosen = i; st.result = null;
    if (inApp) startJob();
    st.busy = true; paint(); renderCenter(); toast('');
    say(label + ': elaborazione in corso.');
    let r;
    if (isMulti()) return execMulti(i, data, a, label);
    try {
      const res = isCustom(a)
        ? await TAURI.core.invoke('run_shortcut', { path: st.file.path, index: a[3].ids[i], nome: a[2][i][0] })
        : await TAURI.core.invoke('run_action', { path: st.file.path, key: keyFor(i), data: data || null });
      const msg = typeof res === 'string' ? res : res.msg, files = (res && res.creati) || [];
      st.result = { ok: true }; r = { ok: true, msg };
      if (!isCustom(a)) saveLast(a, a[2][i]);
      if (endJob().hidden) { finishHidden(msg, files, true); return r; }
      toast(msg, 'ok', files); say(msg);
      if (/^Creato|^Estratto|^Aperto/.test(msg)) closeSoon(files);
    } catch (e) {
      st.result = { ok: false };
      const m = String(e && e.message ? e.message : e);
      r = { ok: false, msg: m };
      if (endJob().hidden) { finishHidden(m, [], false); return r; }
      if (!data || /^Annullato/.test(m)) toast(m, 'err');
      say('Errore: ' + m);
    } finally {
      st.busy = false;
      renderCenter(); paint();
    }
    return r;
  }

  // ---------- lavori lunghi ----------
  function fmtElapsed(ms) { const s = Math.floor(ms / 1000); return Math.floor(s / 60) + ':' + String(s % 60).padStart(2, '0'); }
  function updateJob(c) {
    c = c || slot.querySelector('.center.busy');
    if (!c || !st.job) return;
    const ms = Date.now() - st.job.t0, pct = st.job.pct;
    c.querySelector('.jobline').textContent = (pct != null ? Math.round(pct * 100) + '% · ' : '') + fmtElapsed(ms);
    // «In sottofondo» dopo qualche secondo: per i lavori brevi non serve.
    const bg = c.querySelectorAll('.jobbtn')[1];
    if (bg) bg.hidden = ms < 3000;
  }
  function startJob() {
    st.job = { t0: Date.now(), pct: null, hidden: false, cancel: false };
    st.jobTimer = setInterval(async () => {
      if (!st.job) return;
      try { st.job.pct = await TAURI.core.invoke('job_status'); } catch (e) { /* senza avanzamento */ }
      updateJob();
    }, 500);
  }
  function endJob() { clearInterval(st.jobTimer); const j = st.job; st.job = null; return j || {}; }
  async function cancelJob() {
    if (!st.job || st.job.cancel) return;
    st.job.cancel = true; renderCenter();
    say('Annullo il lavoro.');
    try { await TAURI.core.invoke('job_cancel'); } catch (e) { /* già finito */ }
  }
  // Nasconde la finestra: a lavoro finito arriva una notifica con «Mostra nella cartella».
  async function hideJob() {
    if (!st.job) return;
    st.job.hidden = true;
    try { await TAURI.core.invoke('hide_window'); } catch (e) { /* resta visibile */ }
  }
  // Finito un lavoro nascosto: notifica e chiusura.
  async function finishHidden(msg, files, ok) {
    try { await TAURI.core.invoke('notify_done', { msg, files: files || [], ok }); } catch (e) { /* senza notifica */ }
    closeMenu();
  }

  // Più file: «Insieme» in un colpo solo, le altre azioni un file alla volta con l'avanzamento.
  async function execMulti(i, data, a, label) {
    const key = keyFor(i), n = st.files.length;
    let r;
    try {
      if (isTogether(a)) {
        const msg = await TAURI.core.invoke('run_multi', { paths: st.files.map((f) => f.path), key, data: data || null });
        r = { ok: true, msg };
      } else {
        const lines = [];
        let ok = 0, created = 0;
        r = { files: [] };
        for (let k = 0; k < n; k++) {
          if (st.job && st.job.cancel) { lines.push('Annullato: gli altri ' + (n - k) + ' file non sono stati toccati.'); break; }
          st.progress = (k + 1) + ' di ' + n; renderCenter();
          try {
            const res = await TAURI.core.invoke('run_action', { path: st.files[k].path, key, data: data || null });
            const m = res.msg;
            r.files = r.files.concat(res.creati || []);
            ok++; if (/^Creato|^Estratto/.test(m)) created++;
            lines.push(st.files[k].name + ': ' + m.split('\n')[0]);
          } catch (e) {
            lines.push(st.files[k].name + ': ' + String(e && e.message ? e.message : e));
          }
        }
        const more = lines.length > 6 ? '\n… e altri ' + (lines.length - 6) : '';
        const msg = (ok === n ? 'Fatto su tutti i ' + n + ' file.' : 'Fatto su ' + ok + ' file su ' + n + '.') + '\n' + lines.slice(0, 6).join('\n') + more;
        r = { ok: ok > 0, msg, all: ok === n && created === n, files: r.files };
        if (!ok) throw new Error(msg);
      }
      st.result = { ok: true };
      if (endJob().hidden) { finishHidden(r.msg, r.files, r.ok); return r; }
      toast(r.msg, r.ok && (r.all !== false) ? 'ok' : 'err', r.files); say(r.msg);
      if (r.all || /^Creato/.test(r.msg)) closeLater();
    } catch (e) {
      st.result = { ok: false };
      const m = String(e && e.message ? e.message : e);
      r = { ok: false, msg: m };
      if (endJob().hidden) { finishHidden(m, [], false); return r; }
      if (!data) toast(m, 'err');
      say('Errore: ' + m);
    } finally {
      st.busy = false; st.progress = null;
      renderCenter(); paint();
    }
    return r;
  }

  // ---------- pannello per le opzioni che chiedono dati ----------
  // Tipi di modulo (FORMS in actions.rs): etichette dei campi e testo del pulsante.
  const FORM_UI = {
    'nuova-password': { secret: true, labels: ['Password', 'Ripeti password'], go: 'Proteggi' },
    'password-attuale': { secret: true, labels: ['Password attuale'], go: 'Sblocca' },
    'taglia-inizio': { labels: ['Togli fino a'], go: 'Taglia' },
    'taglia-fine': { labels: ['Tieni fino a'], go: 'Taglia' },
    'dividi': { labels: ['Dividi a'], go: 'Dividi' },
    'autore': { text: true, labels: ['Autore'], go: 'Salva' },
    'titolo': { text: true, labels: ['Titolo'], go: 'Salva' },
    'artista': { text: true, labels: ['Artista'], go: 'Salva' },
    'album': { text: true, labels: ['Album'], go: 'Salva' },
    'data': { date: true, labels: ['Data e ora'], go: 'Salva' },
    'dimensioni': { text: true, labels: ['Larghezza × altezza'], go: 'Ridimensiona',
      help: '1200 = larghezza, 1200x800 = dentro il riquadro, 50% = metà. Le proporzioni restano uguali' },
    'velocita': { text: true, labels: ['Velocità'], go: 'Applica', placeholder: '2 = doppia, 0,5 = metà' },
    'parti': { text: true, labels: ['Dimensione di ogni parte (MB)'], go: 'Dividi', placeholder: '25' },
    'nome-serie-multi': { text: true, labels: ['Nome di base'], go: 'Rinomina', placeholder: 'Vacanze 2026',
      help: 'Si creano delle copie chiamate «Nome 01», «Nome 02»… in ordine alfabetico, in una nuova cartella «Nome»' },
    'nome-serie': { text: true, labels: ['Nome di base'], go: 'Rinomina', placeholder: 'Vacanze 2026',
      help: 'I file diventano «Nome 01», «Nome 02»… nell\'ordine alfabetico, in una copia della cartella' },
    'cerca': { text: true, labels: ['Cerca nei nomi dei file'], go: 'Cerca', placeholder: 'fattura' },
    'trova': { text: true, labels: ['Cerca nel testo'], go: 'Cerca' },
    'sostituisci': { text: true, pair: true, labels: ['Trova', 'Sostituisci con'], go: 'Sostituisci',
      help: 'Testo esatto, maiuscole comprese. Lascia vuoto il secondo campo per togliere il testo' },
    'oscura': { redact: true, labels: [], go: 'Oscura' },
    'oscura-manuale': { redact: true, labels: ['Parole da nascondere'], go: 'Cerca', placeholder: 'Mario Rossi, via Roma 12',
      help: 'Separate da virgole; maiuscole e minuscole non contano' },
    'filigrana': { text: true, labels: ['Testo della filigrana'], go: 'Aggiungi', placeholder: 'BOZZA',
      help: 'In diagonale su ogni pagina, grigio trasparente' },
    'evidenzia': { text: true, labels: ['Parole da evidenziare'], go: 'Evidenzia',
      help: 'Separate da virgole; vengono colorate di giallo le parole che le contengono' },
    'annota-pdf': { text: true, pos: true, pages: true, labels: ['Testo'], go: 'Aggiungi' },
    'annota-img': { text: true, pos: true, labels: ['Testo'], go: 'Aggiungi' },
    'estrai-pagine': { text: true, labels: ['Pagine da tenere'], go: 'Estrai', placeholder: '2, 5-7',
      help: 'Numeri e intervalli separati da virgole: si crea un PDF nuovo solo con quelle pagine' },
    'sottotitoli': { labels: [], go: 'Crea', noOut: true,
      opts: [['Lingua del parlato', 'lingua', [['auto', 'Automatica'], ['it', 'Italiano'], ['en', 'Inglese']], 1],
        ['Modello', 'modello', [['base', 'Veloce'], ['small', 'Preciso']], 0]],
      help: 'Trascrizione automatica con l\'intelligenza artificiale (Whisper), sul computer. La prima volta scarica il modello: Veloce 142 MB, Preciso 466 MB. Può richiedere qualche minuto' },
    'ordine-pagine': { text: true, labels: ['Nuovo ordine delle pagine'], go: 'Riordina', placeholder: '3, 1, 2, 4-10',
      help: 'Numeri e intervalli separati da virgole; le pagine non scritte vengono tolte' },
    'intervallo': { text: true, labels: ['Una foto ogni (secondi)'], go: 'Estrai', placeholder: '1' },
    'scegli': { pick: true, labels: [], go: 'Conferma' },
    'firma': { pick: true, pos: true, pages: true, posDefault: 8, labels: [], go: 'Firma',
      pagesLabels: ['Ultima pagina', 'Tutte'], pagesValues: ['ultima', 'tutte'] },
    'elenco-estrai': { list: 'multi', listLabel: 'File da estrarre', labels: [], go: 'Estrai' },
    'elenco-rimuovi': { list: 'multi', listLabel: 'File da togliere', labels: [], go: 'Togli' },
    'elenco-sostituisci': { list: 'single', listLabel: 'File da sostituire', pick: true, manualPick: true, labels: [], go: 'Sostituisci' },
    'elenco-rinomina': { list: 'single', listLabel: 'File da rinominare', labels: ['Nuovo nome'], go: 'Rinomina' },
    'firma-doc': { pick: true, align: true, labels: [], go: 'Firma', help: 'La firma va in fondo al documento, larga circa 5 cm' },
    'condividi': { share: true, labels: [], go: 'Apri' },
    'ingrandisci': { labels: [], go: 'Upscayl', noOut: true,
      opts: [['Scala', 'scala', [['2', '2×'], ['4', '4×'], ['6', '6×'], ['8', '8×']], 1], ['Tipo di immagine', 'tipo', [['foto', 'Foto'], ['disegni', 'Disegni e grafica']], 0]],
      help: 'Con l\'intelligenza artificiale (Real-ESRGAN, lo stesso motore di Upscayl, sul computer con la scheda video): ricostruisce i dettagli. 6× e 8× fanno due passaggi e richiedono più tempo. Crea «nome_4x», «nome_8x»…' },
    'condividi-link': { share: true, upload: true, labels: [], go: 'Apri' },
    // Editor sull'anteprima (editor.rs): area da trascinare, forma, freccia o punti da cliccare.
    'ed-ritaglia': { editor: 'area', preset: true, go: 'Ritaglia', help: 'Trascina sull\'anteprima l\'area da tenere' },
    'ed-video': { editor: 'area', preset: true, go: 'Ritaglia', help: 'Trascina sul fotogramma l\'area del video da tenere' },
    'ed-sfocato': { editor: 'area', preset: true, shapes: ['Ovale', 'Rettangolo'], go: 'Sfoca', help: 'Trascina sull\'area da tenere nitida: il resto viene sfocato' },
    'ed-forma': { editor: 'forma', shapes: ['Rettangolo', 'Ovale'], colors: 'rosso', go: 'Disegna', help: 'Trascina sull\'anteprima per disegnare la forma' },
    'ed-forma-pdf': { editor: 'forma', shapes: ['Rettangolo', 'Ovale', 'Freccia'], colors: 'rosso', pdf: true, go: 'Disegna', help: 'Trascina sulla pagina per disegnare' },
    'ed-freccia': { editor: 'freccia', colors: 'rosso', go: 'Disegna', help: 'Trascina dalla coda alla punta della freccia' },
    'ed-evidenzia': { editor: 'evidenzia', go: 'Evidenzia', help: 'Trascina sulla parte da evidenziare in giallo' },
    'ed-colore': { editor: 'punti', colors: 'bianco', tol: true, go: 'Applica', help: 'Clicca sullo sfondo: diventa del colore scelto. Ogni clic aggiunge una zona' },
    'ed-rimuovi': { editor: 'punti', tol: true, go: 'Rimuovi', help: 'Clicca sullo sfondo: diventa trasparente. Ogni clic aggiunge una zona' }
  };
  // Stessi nomi e colori di COLORI in editor.rs.
  const ED_COLORS = [['rosso', '#E53935', 'Rosso'], ['giallo', '#FDD835', 'Giallo'], ['verde', '#43A047', 'Verde'],
    ['blu', '#1E88E5', 'Blu'], ['nero', '#111111', 'Nero'], ['bianco', '#FFFFFF', 'Bianco']];
  const ED_TOL = [['bassa', 'Bassa'], ['media', 'Media'], ['alta', 'Alta']];
  function fmtSize(b) {
    if (b >= 1e6) return (b / 1e6).toFixed(1).replace('.', ',') + ' MB';
    return Math.max(1, Math.round(b / 1000)) + ' KB';
  }
  // Elenco dei file di un archivio: caselle (più file) o scelta singola, con filtro se sono tanti.
  function entryList(label, entries, multi, onPick) {
    const g = el('div', 'pgroup', label);
    const box = el('div', 'plist');
    box.setAttribute('role', 'group');
    box.setAttribute('aria-label', label);
    const rows = entries.map(([path, size]) => {
      // Nome del file in evidenza, cartella interna sotto in piccolo.
      const cut = path.lastIndexOf('/');
      const l = el('label'), c = document.createElement('input'), nm = el('span', 'nm');
      nm.appendChild(el('span', null, path.slice(cut + 1)));
      if (cut > 0) nm.appendChild(el('span', 'dir', path.slice(0, cut + 1)));
      c.type = multi ? 'checkbox' : 'radio'; c.name = 'voce'; c.value = path;
      nm.title = path;
      c.addEventListener('change', () => { if (c.checked && onPick) onPick(path); });
      l.append(c, nm, el('span', 'sz', fmtSize(size)));
      box.appendChild(l);
      return { l, c };
    });
    if (entries.length > 10) {
      const f = el('span', 'field'), inp = document.createElement('input');
      inp.type = 'text'; inp.placeholder = 'Filtra per nome…'; inp.autocomplete = 'off'; inp.spellcheck = false;
      inp.setAttribute('aria-label', 'Filtra l\'elenco');
      inp.addEventListener('input', () => {
        const q = inp.value.trim().toLowerCase();
        rows.forEach((r) => { r.l.hidden = !!q && !r.c.value.toLowerCase().includes(q); });
      });
      f.appendChild(inp); g.appendChild(f);
    }
    g.appendChild(box);
    return { box: g, get: () => rows.filter((r) => r.c.checked).map((r) => r.c.value) };
  }
  // Condividi: le app a cui passare lo ZIP, a scelta singola (doppio clic apre subito).
  const SHARE_NOTE = { file: 'Si apre con il file già allegato', incolla: 'Incolla il file con Ctrl+V', trascina: 'Trascina il file nel messaggio',
    link: 'Si apre con il link già scritto' };
  function shareNote(a, isLink) {
    if (a.id === 'cartella') return 'Il file è già selezionato';
    if (a.id === 'browser') return 'Controlla la pagina di download';
    return isLink && a.come === 'incolla' ? 'Incolla il link con Ctrl+V' : SHARE_NOTE[a.come];
  }
  function appList(apps, onOpen, isLink) {
    const g = el('div', 'pgroup', 'Condividi con');
    const box = el('div', 'plist');
    box.setAttribute('role', 'radiogroup');
    box.setAttribute('aria-label', 'Condividi con');
    const rows = apps.map((a, k) => {
      const l = el('label'), c = document.createElement('input'), nm = el('span', 'nm');
      nm.append(el('span', null, a.nome), el('span', 'dir', shareNote(a, isLink)));
      c.type = 'radio'; c.name = 'app'; c.value = a.id; c.checked = k === 0;
      l.append(c, nm);
      l.addEventListener('dblclick', onOpen);
      box.appendChild(l);
      return c;
    });
    g.appendChild(box);
    return { box: g, get: () => { const c = rows.find((r) => r.checked); return c ? c.value : null; } };
  }
  // Testi per le opzioni con «Apri file», in base all'ultima parte della chiave.
  const PICK_TEXT = {
    'archive/contenuto/sostituisci': ['File che prende il suo posto', 'Sostituisci'],
    'unisci': ['Da aggiungere dopo questo', 'Unisci'], 'sostituisci': ['Nuovo audio', 'Sostituisci'],
    'copertina': ['Immagine di copertina', 'Aggiungi'], 'aggiungi-file': ['File da aggiungere', 'Aggiungi'],
    'scegli-dove': ['Cartella di destinazione', 'Estrai'], 'confronta': ['File da confrontare', 'Confronta'], 'firma': ['Immagine della firma (meglio PNG trasparente)', 'Firma']
  };
  const POS_NAMES = ['In alto a sinistra', 'In alto al centro', 'In alto a destra', 'Al centro a sinistra', 'Al centro',
    'Al centro a destra', 'In basso a sinistra', 'In basso al centro', 'In basso a destra'];

  // Gruppo di scelte esclusive (griglia delle posizioni o "Tutte / Solo la prima"): frecce per spostarsi.
  function choices(cls, labels, names, selected, cols, onChange) {
    const box = el('div', cls);
    box.setAttribute('role', 'radiogroup');
    const btns = labels.map((t, k) => {
      const b = el('button', null, t);
      b.type = 'button';
      b.setAttribute('role', 'radio');
      if (names) b.setAttribute('aria-label', names[k]);
      b.addEventListener('click', () => pickChoice(k));
      box.appendChild(b);
      return b;
    });
    let value = selected;
    function pickChoice(k) {
      value = k;
      btns.forEach((b, j) => { b.setAttribute('aria-checked', j === k ? 'true' : 'false'); b.tabIndex = j === k ? 0 : -1; });
      if (onChange) onChange(k);
    }
    box.addEventListener('keydown', (e) => {
      const n = btns.length, step = { ArrowRight: 1, ArrowLeft: -1, ArrowDown: cols, ArrowUp: -cols }[e.key];
      if (!step) return;
      e.preventDefault();
      const k = Math.min(n - 1, Math.max(0, value + step));
      pickChoice(k); btns[k].focus();
    });
    pickChoice(selected);
    return { box, get: () => value };
  }
  const DATE_RE = /^\s*(\d{1,2})[\/.-](\d{1,2})[\/.-](\d{4})(?:\s+(\d{1,2}):(\d{2}))?\s*$/;

  // "90", "1:30", "1:02:30" (anche "1:30,5") → secondi; null se non valido.
  function parseTime(v) {
    const parts = String(v).trim().replace(',', '.').split(':');
    if (!parts[0] || parts.length > 3) return null;
    let total = 0;
    for (let n = 0; n < parts.length; n++) {
      if (!/^\d+(\.\d+)?$/.test(parts[n])) return null;
      const x = parseFloat(parts[n]);
      if ((n < parts.length - 1 && x % 1) || (n > 0 && x >= 60)) return null;
      total = total * 60 + x;
    }
    return total;
  }
  function fmtTime(sec) {
    const t = Math.floor(sec), h = Math.floor(t / 3600), m = Math.floor(t / 60) % 60, s = t % 60;
    const two = (x) => (x < 10 ? '0' : '') + x;
    return h ? h + ':' + two(m) + ':' + two(s) : m + ':' + two(s);
  }

  async function openPanel(i, key, kind, preShared) {
    const a = T().acts[st.level], sub = a[2][i], ui = FORM_UI[kind];
    if (!ui) return;
    if (ui.editor) { await openEditor(i, key, ui); return; }
    st.chosen = i; st.result = null; toast(''); paint(); renderCenter();
    const timeForm = !ui.secret && !ui.text && !ui.date && !ui.redact && !ui.opts;
    let outName = null, dur = null, current = null;
    try { outName = await TAURI.core.invoke('planned_output', { path: st.file.path, key }); } catch (e) { /* solo informativo */ }
    if (timeForm) {
      try { dur = await TAURI.core.invoke('media_duration', { path: st.file.path }); } catch (e) { /* lo controlla il backend */ }
    }
    if ((ui.text || ui.date) && !isMulti()) {
      try { current = await TAURI.core.invoke('current_value', { path: st.file.path, key }); } catch (e) { /* campo vuoto */ }
    }
    // Oscura dati: prima si mostra cosa è stato trovato, poi si conferma.
    const findRedact = async (data) => {
      try {
        const [n, msg] = await TAURI.core.invoke('redact_preview', { path: st.file.path, key, data });
        return { n, msg };
      } catch (e) { return { n: 0, msg: String(e && e.message ? e.message : e), error: true }; }
    };
    let found = null;
    if (ui.redact && !ui.labels.length) found = await findRedact(null);
    let entries = null, entriesErr = null;
    if (ui.list) {
      try { entries = await TAURI.core.invoke('archive_entries', { path: st.file.path }); } catch (e) { entriesErr = String(e && e.message ? e.message : e); }
    }
    // Condividi: lo ZIP si prepara e si copia negli appunti prima di scegliere l'app.
    // Per Link prima si chiede conferma del caricamento online (poi il pannello si riapre con le app).
    let shared = preShared || null, sharedErr = null;
    if (ui.share && !shared && !ui.upload) {
      st.busy = true; paint(); renderCenter(); toast('Preparo lo ZIP da condividere…');
      try { shared = await TAURI.core.invoke('share_prepare', { path: st.file.path, key }); } catch (e) { sharedErr = String(e && e.message ? e.message : e); }
      st.busy = false; toast(''); paint(); renderCenter();
    }
    if (st.chosen !== i || st.level == null) return;

    const p = el('form', 'panel');
    p.noValidate = true;
    p.setAttribute('role', 'dialog');
    p.setAttribute('aria-label', a[0] + ' › ' + sub[0]);
    const head = el('div', 'phead'), pi = el('span', 'picon');
    pi.appendChild(icon(IC[sub[1]], 18, 1.8));
    head.append(pi, el('span', 'ptitle', a[0] + ' › ' + sub[0]));
    p.append(head, el('span', 'fname', st.file.name));

    const field = (label) => {
      const l = el('label', null, label), f = el('span', 'field'), inp = document.createElement('input');
      inp.type = ui.secret ? 'password' : 'text'; inp.autocomplete = 'off'; inp.spellcheck = false;
      if (timeForm) { inp.inputMode = 'decimal'; inp.placeholder = dur ? fmtTime(dur / 2) : '1:30'; }
      if (ui.date) inp.placeholder = '24/10/2026 15:30';
      if (ui.placeholder) inp.placeholder = ui.placeholder;
      if (current) inp.value = current;
      f.appendChild(inp); l.appendChild(f); p.appendChild(l);
      return { inp, f };
    };
    const pw1 = ui.labels[0] ? field(ui.labels[0]) : null;
    const pw2 = ui.labels[1] ? field(ui.labels[1]) : null;
    let posChoice = null, pageChoice = null;
    if (ui.pos) {
      const g = el('div', 'pgroup', 'Posizione');
      posChoice = choices('pos', POS_NAMES.map(() => ''), POS_NAMES, ui.posDefault != null ? ui.posDefault : 2, 3);
      posChoice.box.setAttribute('aria-label', 'Posizione');
      g.appendChild(posChoice.box); p.appendChild(g);
    }
    // Firma nei .docx: va in fondo al documento, allineata a sinistra, al centro o a destra.
    let alignChoice = null;
    if (ui.align) {
      const g = el('div', 'pgroup', 'Allineamento');
      alignChoice = choices('seg', ['Sinistra', 'Centro', 'Destra'], null, 2, 3);
      alignChoice.box.setAttribute('aria-label', 'Allineamento');
      g.appendChild(alignChoice.box); p.appendChild(g);
    }
    // Scelte a segmenti generiche (es. Ingrandisci: scala e tipo di immagine).
    const optChoices = (ui.opts || []).map(([label, k, list, def]) => {
      const g = el('div', 'pgroup', label);
      const c = choices('seg', list.map((x) => x[1]), null, def, list.length);
      c.box.setAttribute('aria-label', label);
      g.appendChild(c.box); p.appendChild(g);
      return [k, list, c];
    });
    if (ui.pages) {
      const g = el('div', 'pgroup', 'Pagine');
      pageChoice = choices('seg', ui.pagesLabels || ['Tutte', 'Solo la prima'], null, 0, 2);
      pageChoice.box.setAttribute('aria-label', 'Pagine');
      g.appendChild(pageChoice.box); p.appendChild(g);
    }
    if (ui.secret) {
      const eye = el('button', 'eye');
      eye.type = 'button'; eye.setAttribute('aria-label', 'Mostra password');
      eye.appendChild(icon(IC.eye, 18));
      pw1.f.appendChild(eye);
      eye.addEventListener('click', () => {
        const show = pw1.inp.type === 'password';
        [pw1, pw2].forEach((x) => { if (x) x.inp.type = show ? 'text' : 'password'; });
        eye.setAttribute('aria-label', show ? 'Nascondi password' : 'Mostra password');
      });
    }

    // Opzioni che usano altri file: pulsante «Scegli…» (si apre da solo la prima volta) e nomi scelti.
    let picked = [], choose = null;
    const pickText = PICK_TEXT[key] || PICK_TEXT[key.split('/').pop()] || ['File scelti', ui.go];
    if (ui.pick) {
      const g = el('div', 'pgroup', pickText[0]);
      const list = el('p', 'picked', 'Nessun file scelto.');
      const btn = el('button', 'ghost pick', 'Scegli…');
      btn.type = 'button';
      g.append(list, btn);
      p.insertBefore(g, p.children[2] || null);
      choose = async () => {
        btn.disabled = true;
        let r = null;
        try { r = await TAURI.core.invoke('pick_files', { path: st.file.path, key }); } catch (e) { /* annullato */ }
        btn.disabled = false;
        if (r && r.length) {
          picked = r;
          list.textContent = r.map((x) => x.split('/').pop()).join(', ');
          btn.textContent = 'Cambia…';
          go.disabled = false; go.focus();
        } else if (!picked.length) btn.focus();
      };
      btn.addEventListener('click', choose);
    }
    // Elenco del contenuto dell'archivio (va sopra gli altri campi).
    let listChoice = null, autoName = '';
    if (ui.list && entries) {
      listChoice = entryList(ui.listLabel, entries, ui.list === 'multi', (path) => {
        // Rinomina: propone il nome attuale, da modificare.
        if (pw1 && (!pw1.inp.value || pw1.inp.value === autoName)) { autoName = path.split('/').pop(); pw1.inp.value = autoName; }
      });
      p.insertBefore(listChoice.box, p.children[2] || null);
    }
    let appChoice = null;
    if (shared) {
      appChoice = appList(shared.app, () => p.requestSubmit(), !!shared.link);
      p.appendChild(appChoice.box);
    }

    const hint = el('p', 'hint');
    if (dur) hint.append('Durata: ', el('b', null, fmtTime(dur)), '. ');
    if (ui.help) hint.append(ui.help + '. ');
    if (outName && kind === 'parti') outName += '.001, .002…';
    if (shared && shared.link) hint.append('Link copiato negli appunti: ', el('b', null, shared.link), '. Chi lo apre può scaricare ', el('b', null, shared.file), '. ');
    // Il link come QR: inquadrandolo con il telefono si apre la pagina di download.
    let qrEl = null;
    if (shared && shared.qr) {
      const q = el('div', 'qrbox'), im = document.createElement('img');
      im.src = 'data:image/svg+xml,' + encodeURIComponent(shared.qr);
      im.alt = 'Codice QR del link';
      q.append(im, el('span', null, 'Inquadralo con il telefono'));
      qrEl = q;
    }
    else if (shared) hint.append('Creato ', el('b', null, shared.file), ' e copiato negli appunti. ');
    else if (ui.upload) hint.append('Halo crea ', el('b', null, outName || 'lo ZIP'), ' e lo carica su ', el('b', null, 'gofile.io'),
      ': chiunque abbia il link potrà scaricarlo. Il caricamento può richiedere qualche minuto. ');
    else if (isMulti()) hint.append(key.startsWith('multi/') ? 'Riguarda tutti i ' + st.files.length + ' file. ' : 'Si applica a ognuno dei ' + st.files.length + ' file: per ciascuno si crea un file nuovo. ');
    else if (outName && !ui.share && !ui.noOut) hint.append('Verrà creato ', el('b', null, outName), kind === 'dividi' ? ' con la parte 2' : '', '. ');
    if (ui.redact && key.startsWith('pdf/')) hint.append('Le pagine diventano immagini: il testo non sarà più selezionabile. ');
    hint.append('L\'originale resta com\'è.');
    const prev = el('p', 'hint');
    const err = el('p', 'perr');
    err.setAttribute('role', 'alert');
    const btns = el('div', 'pbtns'), cancel = el('button', 'ghost', 'Annulla'), go = el('button', 'go', ui.pick ? pickText[1] : ui.upload && !shared ? 'Carica' : ui.go);
    if (ui.pick && !ui.list) go.disabled = true;
    if (entriesErr || sharedErr) go.disabled = true;
    cancel.type = 'button'; go.type = 'submit';
    btns.append(cancel, go);
    if (ui.redact) p.append(prev);
    if (qrEl) p.append(qrEl);
    p.append(hint, err, btns);
    let ready = false;
    const showFound = (f) => {
      prev.textContent = ''; err.textContent = '';
      if (f.error) { err.textContent = f.msg; ready = false; return; }
      prev.append(el('b', null, f.msg));
      ready = f.n > 0;
      go.disabled = !ready && !pw1;
      go.textContent = ready ? 'Oscura' : ui.go;
    };
    if (found) showFound(found);
    if (pw1 && ui.redact) pw1.inp.addEventListener('input', () => { ready = false; prev.textContent = ''; go.textContent = ui.go; });

    cancel.addEventListener('click', () => closePanel(true));
    p.addEventListener('submit', async (e) => {
      e.preventDefault();
      if (st.busy) return;
      const v = pw1 ? pw1.inp.value : '';
      if (ui.upload && !shared) {
        go.disabled = cancel.disabled = true; err.textContent = '';
        st.busy = true; go.textContent = 'Carico…'; say('Caricamento in corso.');
        let r = null;
        try { r = await TAURI.core.invoke('share_prepare', { path: st.file.path, key }); } catch (e2) { err.textContent = String(e2 && e2.message ? e2.message : e2); }
        st.busy = false;
        if (r) { closePanel(false); openPanel(i, key, kind, r); return; }
        go.disabled = cancel.disabled = false; go.textContent = 'Riprova';
        return;
      }
      if (ui.share) {
        const id = appChoice && appChoice.get();
        if (!id) { err.textContent = 'Scegli un\'app dall\'elenco.'; return; }
        go.disabled = cancel.disabled = true;
        try {
          const msg = await TAURI.core.invoke('share_open', { id });
          closePanel(false);
          st.result = { ok: true }; paint(); renderCenter();
          toast(msg, 'ok'); say(msg);
          closeLater();
        } catch (e2) {
          go.disabled = cancel.disabled = false;
          err.textContent = String(e2 && e2.message ? e2.message : e2);
        }
        return;
      }
      let data;
      if (ui.list) {
        const voci = listChoice ? listChoice.get() : [];
        if (!voci.length) { err.textContent = 'Scegli almeno un file dall\'elenco.'; return; }
        data = { voci: voci.join('\n') };
        if (ui.pick) {
          if (!picked.length) { err.textContent = 'Scegli il file che prende il suo posto.'; return; }
          data.file = picked[0];
        }
        if (pw1) {
          if (!v.trim()) { err.textContent = 'Scrivi il nuovo nome.'; pw1.inp.focus(); return; }
          data.valore = v.trim();
        }
      } else if (ui.pick) {
        if (!picked.length) { err.textContent = 'Scegli prima i file.'; return; }
        data = { file: picked.join('\n') };
        if (posChoice) data.posizione = String(posChoice.get());
        if (pageChoice) data.pagine = ui.pagesValues[pageChoice.get()];
        if (alignChoice) data.allineamento = ['sinistra', 'centro', 'destra'][alignChoice.get()];
      } else if (ui.redact) {
        if (pw1 && !v.trim()) { err.textContent = 'Scrivi le parole da nascondere.'; pw1.inp.focus(); return; }
        data = pw1 ? { valore: v } : {};
        if (!ready) {
          go.disabled = true;
          showFound(await findRedact(data));
          go.disabled = false;
          return;
        }
      } else if (ui.secret) {
        if (!v) { err.textContent = 'Scrivi una password.'; pw1.inp.focus(); return; }
        if (pw2 && v !== pw2.inp.value) { err.textContent = 'Le due password non coincidono.'; pw2.inp.focus(); pw2.inp.select(); return; }
        data = { password: v };
      } else if (ui.pair) {
        if (!v) { err.textContent = 'Scrivi il testo da cercare.'; pw1.inp.focus(); return; }
        data = { valore: v, sostituto: pw2.inp.value };
      } else if (ui.text) {
        if (!v.trim()) { err.textContent = 'Scrivi un valore.'; pw1.inp.focus(); return; }
        data = { valore: v.trim() };
        if (posChoice) data.posizione = String(posChoice.get());
        if (pageChoice) data.pagine = pageChoice.get() === 1 ? 'prima' : 'tutte';
      } else if (ui.opts) {
        data = {};
        optChoices.forEach(([k, list, c]) => { data[k] = list[c.get()][0]; });
      } else if (ui.date) {
        if (!DATE_RE.test(v)) { err.textContent = 'Scrivi la data come 24/10/2026 o 24/10/2026 15:30.'; pw1.inp.focus(); pw1.inp.select(); return; }
        data = { valore: v.trim() };
      } else {
        const sec = parseTime(v);
        if (sec == null) { err.textContent = 'Scrivi un orario, per esempio 1:30.'; pw1.inp.focus(); pw1.inp.select(); return; }
        if (dur && (sec <= 0 || sec >= dur)) { err.textContent = 'L\'orario deve essere tra 0:00 e ' + fmtTime(dur) + '.'; pw1.inp.focus(); pw1.inp.select(); return; }
        data = { orario: String(sec) };
      }
      err.textContent = '';
      go.disabled = cancel.disabled = true;
      const r = await execAction(i, data);
      if (r.ok) { closePanel(false); return; }
      go.disabled = cancel.disabled = false;
      err.textContent = r.msg;
      if (pw1) { pw1.inp.focus(); pw1.inp.select(); } else cancel.focus();
    });

    document.body.appendChild(p);
    st.panel = p;
    placePanel(p);
    if (entriesErr || sharedErr) err.textContent = entriesErr || sharedErr;
    const firstBox = (listChoice || appChoice) && (listChoice || appChoice).box.querySelector('input');
    if (firstBox) firstBox.focus();
    else if (pw1) { pw1.inp.focus(); if (current) pw1.inp.select(); } else (go.disabled ? cancel : go).focus();
    if (choose && !ui.manualPick) choose();
    say(a[0] + ' › ' + sub[0] + ': ' + (ui.share ? (shared ? (shared.link ? 'il link è negli appunti' : shared.file + ' è negli appunti') + '; scegli l\'app e premi Invio' : ui.upload ? 'premi Invio per caricare la cartella online' : sharedErr) : ui.list ? 'scegli i file dall\'elenco' : ui.pick ? 'scegli i file nella finestra che si apre' : ui.redact ? (found ? found.msg + ' Invio per oscurare' : 'scrivi le parole da nascondere e premi Invio per cercarle')
      : (ui.opts ? 'scegli le opzioni' : ui.secret ? 'scrivi la password' : ui.text ? 'scrivi il nuovo valore' : ui.date ? 'scrivi la data' : 'scrivi un orario') + ' e premi Invio') + '. Esc per annullare.');
  }

  // ---------- Traduci: prompt pronto per uno strumento di IA ----------
  async function openTranslate(i) {
    // Azione diretta dal primo livello: lo spicchio Traduci apre subito il pannello.
    const a = T().acts[i], key = T().kind + '/' + slug(a[0]);
    st.result = null; toast('');
    let tools = [], langs = [];
    try { [tools, langs] = await TAURI.core.invoke('translate_tools'); } catch (e) { /* elenco vuoto */ }
    if (st.level != null || st.panel) return;
    const p = el('form', 'panel');
    p.noValidate = true;
    p.setAttribute('role', 'dialog');
    p.setAttribute('aria-label', a[0]);
    const head = el('div', 'phead'), pi = el('span', 'picon');
    pi.appendChild(icon(IC[a[1]], 18, 1.8));
    head.append(pi, el('span', 'ptitle', a[0]));
    p.append(head, el('span', 'fname', st.file.name));

    // Lingua: l'ultima usata resta scelta per la volta dopo.
    let last = 'inglese';
    try { last = localStorage.getItem('halo-lingua') || last; } catch (e) { /* senza memoria */ }
    const ll = el('label', null, 'In quale lingua'), lf = el('span', 'field'), sel = document.createElement('select');
    langs.forEach(([id, nome]) => { const o = document.createElement('option'); o.value = id; o.textContent = nome; sel.appendChild(o); });
    if (langs.some(([id]) => id === last)) sel.value = last;
    lf.appendChild(sel); ll.appendChild(lf);
    p.appendChild(ll);

    const g = el('div', 'pgroup', 'Traduci con');
    const box = el('div', 'plist');
    box.setAttribute('role', 'radiogroup');
    box.setAttribute('aria-label', 'Traduci con');
    const radios = tools.map(([id, nome], k) => {
      const l = el('label'), c = document.createElement('input'), nm = el('span', 'nm');
      nm.append(el('span', null, nome), el('span', 'dir', 'Si apre nel browser'));
      c.type = 'radio'; c.name = 'ia'; c.value = id; c.checked = k === 0;
      l.append(c, nm);
      l.addEventListener('dblclick', () => p.requestSubmit());
      box.appendChild(l);
      return c;
    });
    g.appendChild(box);
    p.appendChild(g);
    const gf = el('div', 'pgroup', 'Ricevi il file come');
    // Di partenza lo stesso formato del file: PDF per i PDF, Word per i documenti.
    const fmt = choices('seg', ['Word (.docx)', 'PDF'], null, /\.pdf$/i.test(st.file.name) ? 1 : 0, 2);
    fmt.box.setAttribute('aria-label', 'Ricevi il file come');
    gf.appendChild(fmt.box);
    p.appendChild(gf);
    p.append(el('p', 'hint', 'Halo copia negli appunti il prompt per la traduzione e mostra il file nella sua cartella: ' +
      'nella chat incolla il prompt (Ctrl+V) e trascina il file. La traduzione la fa lo strumento scelto, online: ' +
      'il documento viene inviato a quel servizio.'));
    const err = el('p', 'perr');
    err.setAttribute('role', 'alert');
    const btns = el('div', 'pbtns'), cancel = el('button', 'ghost', 'Annulla'), go = el('button', 'go', 'Apri');
    cancel.type = 'button'; go.type = 'submit';
    btns.append(cancel, go);
    p.append(err, btns);
    if (!tools.length || !langs.length) { err.textContent = 'Nessuno strumento disponibile.'; go.disabled = true; }
    cancel.addEventListener('click', () => closePanel(true));
    p.addEventListener('submit', async (e) => {
      e.preventDefault();
      const c = radios.find((r) => r.checked);
      if (!c) { err.textContent = 'Scegli lo strumento.'; return; }
      go.disabled = cancel.disabled = true;
      try { localStorage.setItem('halo-lingua', sel.value); } catch (e2) { /* senza memoria */ }
      try {
        const msg = await TAURI.core.invoke('translate_open', { path: st.file.path, key, lingua: sel.value, tool: c.value, formato: fmt.get() === 1 ? 'pdf' : 'docx' });
        closePanel(false);
        toast(msg, 'ok'); say(msg);
        closeLater();
      } catch (e2) {
        go.disabled = cancel.disabled = false;
        err.textContent = String(e2 && e2.message ? e2.message : e2);
      }
    });
    document.body.appendChild(p);
    st.panel = p;
    placePanel(p);
    sel.focus();
    say(a[0] + ': scegli la lingua e lo strumento di intelligenza artificiale, poi premi Invio. Esc per annullare.');
  }

  // ---------- editor sull'anteprima ----------
  async function openEditor(i, key, ui) {
    const a = T().acts[st.level], sub = a[2][i];
    st.chosen = i; st.result = null; paint(); renderCenter();
    let page = 1, prev = null, prevErr = null, outName = null;
    const load = async (data) => {
      try { return await TAURI.core.invoke('editor_preview', { path: st.file.path, key, page, data: data || null }); } catch (e) { prevErr = String(e && e.message ? e.message : e); return null; }
    };
    try { outName = await TAURI.core.invoke('planned_output', { path: st.file.path, key }); } catch (e) { /* solo informativo */ }
    st.busy = true; paint(); renderCenter(); toast('Preparo l\'anteprima…');
    prev = await load(null);
    st.busy = false; toast(''); paint(); renderCenter();
    if (st.chosen !== i || st.level == null) return;

    const p = el('form', 'panel editor');
    p.noValidate = true;
    p.setAttribute('role', 'dialog');
    p.setAttribute('aria-label', a[0] + ' › ' + sub[0]);
    const head = el('div', 'phead'), pi = el('span', 'picon');
    pi.appendChild(icon(IC[sub[1]], 18, 1.8));
    head.append(pi, el('span', 'ptitle', a[0] + ' › ' + sub[0]));
    p.append(head, el('span', 'fname', st.file.name));

    // Anteprima con la tela per disegnare sopra.
    const stage = el('div', 'ed-stage'), wrap = el('div', 'ed-wrap'), img = document.createElement('img'), cv = document.createElement('canvas');
    img.alt = 'Anteprima di ' + st.file.name; img.draggable = false;
    cv.setAttribute('aria-hidden', 'true');
    if (ui.editor === 'punti' && key.endsWith('/rimuovi')) wrap.classList.add('checker');
    wrap.append(img, cv); stage.appendChild(wrap); p.appendChild(stage);
    let original = prev ? prev.img : '';
    if (prev) img.src = prev.img;

    // Pagine del PDF: ‹ Pagina 1 di 5 ›
    let pageLab = null;
    if (ui.pdf && prev && prev.pagine > 1) {
      const nav = el('div', 'ed-pages'), back = el('button', 'ghost', '‹'), fwd = el('button', 'ghost', '›');
      pageLab = el('span', null, '');
      back.type = fwd.type = 'button';
      back.setAttribute('aria-label', 'Pagina precedente'); fwd.setAttribute('aria-label', 'Pagina successiva');
      const go2 = async (d) => {
        const np = Math.min(prev.pagine, Math.max(1, page + d));
        if (np === page) return;
        page = np; area = null; draw(); showPage();
        const r = await load(null);
        if (r) { prev = r; original = r.img; img.src = r.img; }
      };
      back.addEventListener('click', () => go2(-1)); fwd.addEventListener('click', () => go2(1));
      nav.append(back, pageLab, fwd); p.appendChild(nav);
    }
    const showPage = () => { if (pageLab) pageLab.textContent = 'Pagina ' + page + ' di ' + prev.pagine; };
    showPage();

    // Strumenti: forma, colore, tolleranza.
    let shapeChoice = null, colorChoice = null, tolChoice = null, area = null, points = [];
    const row = el('div', 'ed-tools');
    if (ui.shapes) {
      const g = el('div', 'pgroup', 'Forma');
      shapeChoice = choices('seg', ui.shapes, null, 0, ui.shapes.length, () => draw());
      shapeChoice.box.setAttribute('aria-label', 'Forma');
      g.appendChild(shapeChoice.box); row.appendChild(g);
    }
    if (ui.colors) {
      const g = el('div', 'pgroup', 'Colore');
      const start = ED_COLORS.findIndex((c) => c[0] === ui.colors);
      colorChoice = choices('swatch', ED_COLORS.map(() => ''), ED_COLORS.map((c) => c[2]), start, 1, () => { draw(); refreshFill(); });
      Array.from(colorChoice.box.children).forEach((b, k) => { b.style.background = ED_COLORS[k][1]; });
      colorChoice.box.setAttribute('aria-label', 'Colore');
      g.appendChild(colorChoice.box); row.appendChild(g);
    }
    if (ui.tol) {
      const g = el('div', 'pgroup', 'Tolleranza');
      tolChoice = choices('seg', ED_TOL.map((x) => x[1]), null, 1, 3, () => refreshFill());
      tolChoice.box.setAttribute('aria-label', 'Tolleranza');
      g.appendChild(tolChoice.box); row.appendChild(g);
    }
    let reset = null;
    if (ui.editor === 'punti') {
      reset = el('button', 'ghost pick', 'Ricomincia');
      reset.type = 'button'; reset.disabled = true;
      reset.addEventListener('click', () => { points = []; reset.disabled = true; img.src = original; draw(); });
      row.appendChild(reset);
    }
    if (row.children.length) p.appendChild(row);

    const hint = el('p', 'hint'), size = el('b');
    if (ui.help) hint.append(ui.help + '. ');
    if (ui.editor === 'area' && !ui.shapes) hint.append('Area: ', size, '. ');
    if (outName) hint.append('Verrà creato ', el('b', null, outName), '. ');
    hint.append('L\'originale resta com\'è.');
    const err = el('p', 'perr');
    err.setAttribute('role', 'alert');
    const btns = el('div', 'pbtns'), cancel = el('button', 'ghost', 'Annulla'), go = el('button', 'go', ui.go);
    cancel.type = 'button'; go.type = 'submit';
    btns.append(cancel, go);
    p.append(hint, err, btns);
    if (!prev) { err.textContent = prevErr; go.disabled = true; }

    const shape = () => (shapeChoice ? ui.shapes[shapeChoice.get()].toLowerCase() : ui.editor);
    const colorHex = () => (colorChoice ? ED_COLORS[colorChoice.get()][1] : '#FFE94D');
    if (ui.preset) area = [0.15, 0.15, 0.85, 0.85];

    // Disegno sulla tela (coordinate in frazioni 0–1 dell'anteprima).
    function draw() {
      const W = img.clientWidth, H = img.clientHeight, r = window.devicePixelRatio || 1;
      if (!W || !H) return;
      cv.width = Math.round(W * r); cv.height = Math.round(H * r);
      const c = cv.getContext('2d');
      c.setTransform(r, 0, 0, r, 0, 0);
      c.clearRect(0, 0, W, H);
      points.forEach(([x, y]) => {
        c.beginPath(); c.arc(x * W, y * H, 6, 0, Math.PI * 2);
        c.fillStyle = '#FFFFFF'; c.fill(); c.lineWidth = 2.5; c.strokeStyle = '#C2410C'; c.stroke();
      });
      if (size.isConnected) size.textContent = area && prev ? Math.round(Math.abs(area[2] - area[0]) * prev.w) + ' × ' + Math.round(Math.abs(area[3] - area[1]) * prev.h) + ' px' : '—';
      if (!area) return;
      const x0 = Math.min(area[0], area[2]) * W, y0 = Math.min(area[1], area[3]) * H;
      const w = Math.abs(area[2] - area[0]) * W, h = Math.abs(area[3] - area[1]) * H;
      const oval = (ctx) => { ctx.beginPath(); ctx.ellipse(x0 + w / 2, y0 + h / 2, w / 2, h / 2, 0, 0, Math.PI * 2); };
      const s = shape();
      if (ui.editor === 'area') {
        // Fuori dall'area: velo scuro; dentro: com'è, con il bordo tratteggiato.
        c.fillStyle = 'rgba(0, 0, 0, 0.5)'; c.fillRect(0, 0, W, H);
        c.globalCompositeOperation = 'destination-out';
        if (s === 'ovale') { oval(c); c.fill(); } else c.fillRect(x0, y0, w, h);
        c.globalCompositeOperation = 'source-over';
        c.setLineDash([6, 4]); c.lineWidth = 1.5; c.strokeStyle = '#FFFFFF';
        if (s === 'ovale') { oval(c); c.stroke(); } else c.strokeRect(x0, y0, w, h);
        c.setLineDash([]);
      } else if (ui.editor === 'evidenzia') {
        c.globalCompositeOperation = 'multiply'; c.fillStyle = colorHex(); c.fillRect(x0, y0, w, h);
        c.globalCompositeOperation = 'source-over';
      } else if (s === 'freccia') {
        const ax = area[0] * W, ay = area[1] * H, bx = area[2] * W, by = area[3] * H, len = Math.hypot(bx - ax, by - ay);
        if (len < 4) return;
        const ux = (bx - ax) / len, uy = (by - ay) / len, hl = Math.min(15, len * 0.6), hw = hl * 0.6;
        c.strokeStyle = c.fillStyle = colorHex(); c.lineWidth = 3; c.lineCap = 'round';
        c.beginPath(); c.moveTo(ax, ay); c.lineTo(bx - ux * hl * 0.8, by - uy * hl * 0.8); c.stroke();
        c.beginPath(); c.moveTo(bx, by);
        c.lineTo(bx - ux * hl - uy * hw, by - uy * hl + ux * hw); c.lineTo(bx - ux * hl + uy * hw, by - uy * hl - ux * hw);
        c.closePath(); c.fill();
      } else {
        c.strokeStyle = colorHex(); c.lineWidth = 3;
        if (s === 'ovale') { oval(c); c.stroke(); } else c.strokeRect(x0, y0, w, h);
      }
    }
    img.addEventListener('load', () => { draw(); placePanel(p); });

    // Sfondo › Colore e Rimuovi: l'anteprima mostra subito il risultato dei clic.
    let fillSeq = 0;
    async function refreshFill() {
      if (ui.editor !== 'punti' || !points.length) return;
      const n = ++fillSeq;
      const r = await load(fillData());
      if (n !== fillSeq) return;
      if (r) { img.src = r.img; err.textContent = ''; } else err.textContent = prevErr;
    }
    const fillData = () => {
      const d = { punti: points.map(([x, y]) => x.toFixed(4) + ',' + y.toFixed(4)).join(';') };
      if (colorChoice) d.colore = ED_COLORS[colorChoice.get()][0];
      if (tolChoice) d.tolleranza = ED_TOL[tolChoice.get()][0];
      return d;
    };

    const at = (e) => {
      const b = cv.getBoundingClientRect();
      return [Math.min(1, Math.max(0, (e.clientX - b.left) / b.width)), Math.min(1, Math.max(0, (e.clientY - b.top) / b.height))];
    };
    let dragFrom = null;
    cv.addEventListener('pointerdown', (e) => {
      if (!prev || st.busy) return;
      e.preventDefault();
      const q = at(e);
      if (ui.editor === 'punti') {
        points.push(q); reset.disabled = false; draw(); refreshFill();
        return;
      }
      cv.setPointerCapture(e.pointerId);
      dragFrom = q; area = [q[0], q[1], q[0], q[1]]; err.textContent = ''; draw();
    });
    cv.addEventListener('pointermove', (e) => {
      if (!dragFrom) return;
      const q = at(e);
      area = [dragFrom[0], dragFrom[1], q[0], q[1]]; draw();
    });
    const end = () => { dragFrom = null; };
    cv.addEventListener('pointerup', end); cv.addEventListener('pointercancel', end);

    cancel.addEventListener('click', () => closePanel(true));
    p.addEventListener('submit', async (e) => {
      e.preventDefault();
      if (st.busy || !prev) return;
      let data;
      if (ui.editor === 'punti') {
        if (!points.length) { err.textContent = 'Clicca sullo sfondo nell\'anteprima.'; return; }
        data = fillData();
      } else {
        if (!area) { err.textContent = 'Disegna prima sull\'anteprima.'; return; }
        data = { area: area.map((v) => v.toFixed(4)).join(',') };
        if (shapeChoice) data.forma = shape();
        if (colorChoice) data.colore = ED_COLORS[colorChoice.get()][0];
        if (ui.pdf) data.pagina = String(page);
      }
      err.textContent = '';
      go.disabled = cancel.disabled = true;
      const r = await execAction(i, data);
      if (r.ok) { closePanel(false); return; }
      go.disabled = cancel.disabled = false;
      err.textContent = r.msg;
    });

    document.body.appendChild(p);
    st.panel = p;
    placePanel(p);
    draw();
    (go.disabled ? cancel : go).focus();
    say(a[0] + ' › ' + sub[0] + ': ' + (prev ? (ui.help || '') + '. Invio per applicare' : prevErr) + '. Esc per annullare.');
  }

  // Margine tra il riquadro del menu (MENU px) e il menu visibile, rimpicciolito o ingrandito dalle impostazioni.
  function menuInset() { return MENU * (1 - (parseFloat(css('--menu-scale')) || 1)) / 2; }

  function placePanel(p) {
    const W = window.innerWidth, H = window.innerHeight, w = p.offsetWidth, h = p.offsetHeight, gap = 4, off = menuInset();
    const mx = parseFloat(menu.style.left) || 0, my = parseFloat(menu.style.top) || 0;
    let x = mx + MENU - off + gap;
    if (x + w > W - 12) x = mx + off - w - gap;
    if (x < 12) x = Math.max(12, Math.min(mx + (MENU - w) / 2, W - w - 12));
    const y = Math.max(12, Math.min(my + MENU / 2 - h / 2, H - h - 12));
    p.style.left = x + 'px'; p.style.top = y + 'px';
  }

  function closePanel(refocus) {
    if (!st.panel) return;
    // Impostazioni non salvate: si torna all'aspetto di prima.
    if (st.panel.revert) st.panel.revert();
    st.panel.remove();
    st.panel = null;
    if (refocus) {
      st.chosen = null; paint(); renderCenter();
      focusWedge(st.focus);
    }
  }

  // ---------- impostazioni: aspetto ----------
  const GEAR = 'M12 9a3 3 0 1 0 0 6a3 3 0 1 0 0-6zM19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z';
  // Colori pronti: tutti con abbastanza contrasto per il testo bianco sopra.
  const SET_COLORS = [['#C2410C', 'Arancio'], ['#DC2626', 'Rosso'], ['#BE185D', 'Rosa'], ['#A21CAF', 'Fucsia'], ['#7C3AED', 'Viola'],
    ['#4F46E5', 'Indaco'], ['#1D5FD1', 'Blu'], ['#0369A1', 'Azzurro'], ['#0F766E', 'Verde acqua'], ['#15803D', 'Verde'],
    ['#4D7C0F', 'Oliva'], ['#92400E', 'Marrone'], ['#374151', 'Grafite'], ['#111827', 'Nero']];
  const SET_THEME = [['auto', 'Automatico'], ['chiaro', 'Chiaro'], ['scuro', 'Scuro']];
  const SET_GLASS = [['leggero', 'Leggero', 0.72], ['normale', 'Normale', 1], ['pieno', 'Pieno', 1.13]];
  const SET_SIZE = [['piccolo', 'Piccolo', 0.85], ['normale', 'Normale', 1], ['grande', 'Grande', 1.15]];
  const SET_ZOOM = [['nessuno', 'Nessuno', 1], ['lieve', 'Lieve', 1.04], ['normale', 'Normale', 1.07], ['forte', 'Forte', 1.11]];
  const SET_ANIM = [['normali', 'Normali'], ['ridotte', 'Ridotte']];
  const SET_CLOSE = [['3', '3 s'], ['5', '5 s'], ['10', '10 s'], ['mai', 'Mai']];
  const SET_DEFAULT = { colore: '#C2410C', tema: 'auto', vetro: 'normale', grandezza: 'normale', ingrandimento: 'normale', animazioni: 'normali', chiusura: '5' };
  let settings = { aspetto: Object.assign({}, SET_DEFAULT) };

  function hexRgb(h) { return [1, 3, 5].map((k) => parseInt(h.slice(k, k + 2), 16)); }
  // Contrasto con il testo bianco (WCAG): sotto 3 le etichette sul colore non si leggono.
  function whiteContrast(h) {
    const L = hexRgb(h).map((c) => { c /= 255; return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4); });
    return 1.05 / (0.2126 * L[0] + 0.7152 * L[1] + 0.0722 * L[2] + 0.05);
  }
  function applySettings(a) {
    a = Object.assign({}, SET_DEFAULT, a || {});
    const r = document.documentElement.style, c = a.colore.toUpperCase();
    // Il colore scuro (spicchio scelto) è lo stesso colore all'82%: per l'arancio dà il #9F350A approvato.
    const deep = c === '#C2410C' ? '#9F350A' : '#' + hexRgb(c).map((x) => Math.round(x * 0.82).toString(16).padStart(2, '0')).join('');
    r.setProperty('--accent', c); r.setProperty('--accent-deep', deep);
    r.setProperty('--glass-k', String((SET_GLASS.find((g) => g[0] === a.vetro) || SET_GLASS[1])[2]));
    r.setProperty('--menu-scale', String((SET_SIZE.find((g) => g[0] === a.grandezza) || SET_SIZE[1])[2]));
    SCALE_HOT = (SET_ZOOM.find((g) => g[0] === a.ingrandimento) || SET_ZOOM[2])[2];
    document.documentElement.classList.toggle('reduced', a.animazioni === 'ridotte');
    if (a.tema === 'chiaro') document.documentElement.dataset.theme = 'light';
    else if (a.tema === 'scuro') document.documentElement.dataset.theme = 'dark';
    else delete document.documentElement.dataset.theme;
    if (refs.fills.length) paint();
  }

  // `start`: valori da mostrare (quelli salvati, o i predefiniti dopo «Ripristina»).
  function openSettings(start) {
    if (st.busy) return;
    closePanel(true);
    const saved = Object.assign({}, SET_DEFAULT, settings.aspetto), draft = Object.assign({}, start && start.colore ? start : saved);
    const p = el('form', 'panel settings');
    p.noValidate = true;
    p.setAttribute('role', 'dialog');
    p.setAttribute('aria-label', 'Impostazioni');
    const head = el('div', 'phead'), pi = el('span', 'picon');
    pi.appendChild(icon(GEAR, 18, 1.8));
    head.append(pi, el('span', 'ptitle', 'Impostazioni › Aspetto'));
    p.append(head, settingsTabs(0));
    const preview = () => { applySettings(draft); if (p.isConnected) placePanel(p); };
    const err = el('p', 'perr');
    err.setAttribute('role', 'alert');

    const g1 = el('div', 'pgroup', 'Colore');
    const cl = el('label', null, 'Altro colore'), cf = el('span', 'field'), custom = document.createElement('input');
    custom.type = 'text'; custom.placeholder = '#2563EB'; custom.maxLength = 7; custom.spellcheck = false; custom.autocomplete = 'off';
    const ci = SET_COLORS.findIndex((x) => x[0] === draft.colore);
    if (ci < 0) custom.value = draft.colore;
    const colors = choices('swatch', SET_COLORS.map(() => ''), SET_COLORS.map((x) => x[1]), ci, 1, (k) => {
      if (!SET_COLORS[k]) return;
      draft.colore = SET_COLORS[k][0]; custom.value = ''; err.textContent = ''; preview();
    });
    Array.from(colors.box.children).forEach((b, k) => { b.style.background = SET_COLORS[k][0]; });
    colors.box.setAttribute('aria-label', 'Colore');
    cf.appendChild(custom); cl.appendChild(cf);
    g1.append(colors.box, cl);
    p.appendChild(g1);
    custom.addEventListener('input', () => {
      const v = custom.value.trim();
      if (!/^#?[0-9a-fA-F]{6}$/.test(v)) { err.textContent = v.length >= 6 ? 'Scrivi il colore come #2563EB.' : ''; return; }
      const h = (v[0] === '#' ? v : '#' + v).toUpperCase();
      if (whiteContrast(h) < 3) { err.textContent = 'Colore troppo chiaro: il testo bianco sopra non si leggerebbe.'; return; }
      err.textContent = '';
      Array.from(colors.box.children).forEach((b) => { b.setAttribute('aria-checked', 'false'); });
      draft.colore = h; preview();
    });

    const seg = (label, list, key) => {
      const g = el('div', 'pgroup', label);
      const c = choices('seg', list.map((x) => x[1]), null, Math.max(0, list.findIndex((x) => x[0] === draft[key])), list.length,
        (k) => { draft[key] = list[k][0]; preview(); });
      c.box.setAttribute('aria-label', label);
      g.appendChild(c.box); p.appendChild(g);
    };
    seg('Tema', SET_THEME, 'tema');
    seg('Vetro', SET_GLASS, 'vetro');
    seg('Grandezza del menu', SET_SIZE, 'grandezza');
    seg('Spicchio sotto il cursore', SET_ZOOM, 'ingrandimento');
    seg('Animazioni', SET_ANIM, 'animazioni');
    seg('Chiusura dopo un\'operazione', SET_CLOSE, 'chiusura');

    const btns = el('div', 'pbtns'), reset = el('button', 'ghost', 'Ripristina'), cancel = el('button', 'ghost', 'Annulla'), go = el('button', 'go', 'Salva');
    reset.type = cancel.type = 'button'; go.type = 'submit';
    reset.style.marginRight = 'auto';
    btns.append(reset, cancel, go);
    p.append(el('p', 'hint', 'Le modifiche si vedono subito; «Salva» le tiene anche per le prossime aperture.'), err, btns);

    // Senza «Salva» si torna all'aspetto salvato.
    p.revert = () => applySettings(saved);
    reset.addEventListener('click', () => openSettings(SET_DEFAULT));
    cancel.addEventListener('click', () => closePanel(true));
    p.addEventListener('submit', async (e) => {
      e.preventDefault();
      if (err.textContent) { custom.focus(); return; }
      go.disabled = cancel.disabled = true;
      try {
        if (inApp) settings = await TAURI.core.invoke('save_settings', { settings: { aspetto: draft } });
        else settings.aspetto = Object.assign({}, draft);
      } catch (e2) {
        err.textContent = String(e2 && e2.message ? e2.message : e2);
        go.disabled = cancel.disabled = false;
        return;
      }
      p.revert = null;
      closePanel(true);
      applySettings(settings.aspetto);
      toast('Impostazioni salvate.', 'ok'); say('Impostazioni salvate.');
    });

    document.body.appendChild(p);
    st.panel = p;
    preview();
    (colors.box.querySelector('[tabindex="0"]') || custom).focus();
    say('Impostazioni: colore, tema, vetro, grandezza, spicchio attivo, animazioni e chiusura. Le modifiche si vedono subito. Esc per annullare.');
  }

  // ---------- impostazioni: scorciatoie («Le mie app») ----------
  const SC_TIPI = [['image', 'Immagini'], ['pdf', 'PDF'], ['audio', 'Audio'], ['video', 'Video'], ['document', 'Documenti'],
    ['sheet', 'Fogli'], ['text', 'Testi'], ['archive', 'Archivi'], ['folder', 'Cartelle'], ['slides', 'Presentazioni'], ['other', 'Altri file']];
  const SC_MODI = [['apri', 'Apri il file'], ['comando', 'Comando'], ['upscayl', 'Ingrandisci 4×']];
  const SC_ICON = { apri: 'share', comando: 'code', upscayl: 'resize' };
  const SC_MAX_MENU = 6;

  // Aggiunge (o toglie) lo spicchio «Le mie app» nei tipi di file che hanno scorciatoie.
  function applyShortcuts() {
    const list = (settings.scorciatoie || []);
    TYPES.forEach((t) => {
      if (t.multi) return;
      t.acts = t.acts.filter((a) => !isCustom(a));
      const mine = list.map((s, k) => [s, k]).filter(([s]) => s.tipi.indexOf(t.kind) >= 0).slice(0, SC_MAX_MENU);
      if (mine.length) t.acts.push(['Le mie app', 'sparkle', mine.map(([s]) => [s.nome, SC_ICON[s.modo] || 'share']), { custom: true, ids: mine.map(([, k]) => k) }]);
    });
  }
  function settingsTabs(k) {
    const c = choices('seg', ['Aspetto', 'Scorciatoie', 'Cronologia'], null, k, 3, (j) => {
      if (j === k) return;
      if (j === 0) openSettings(); else if (j === 1) openShortcuts(); else openHistory();
    });
    c.box.setAttribute('aria-label', 'Sezione delle impostazioni');
    return c.box;
  }
  // ---------- impostazioni: cronologia ----------
  // «PDF › Comprimi › Per email» dalla chiave «pdf/comprimi/per-email».
  function labelForKey(key) {
    const [kind, act, sub] = key.split('/');
    if (kind === 'multi') {
      const s = { zip: 'ZIP', 'per-email': 'Per email', 'rinomina-in-serie': 'Rinomina in serie', 'data-di-scatto': 'Data di scatto',
        unisci: 'Unisci', 'unisci-in-pdf': 'Unisci in PDF', percorsi: 'Percorsi' }[sub] || sub;
      return 'Più file › Insieme › ' + s;
    }
    const t = TYPES.find((x) => x.kind === kind && !x.multi);
    const a = t && t.acts.find((x) => slug(x[0]) === act);
    if (!a) return key;
    const s = sub && a[2].find((x) => slug(x[0]) === sub);
    return t.tab + ' › ' + a[0] + (s ? ' › ' + s[0] : '');
  }
  function fmtWhen(sec) {
    const d = new Date(sec * 1000), now = new Date(), two = (x) => String(x).padStart(2, '0');
    const hm = two(d.getHours()) + ':' + two(d.getMinutes());
    const day = (x) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
    const diff = Math.round((day(now) - day(d)) / 86400000);
    return diff === 0 ? 'oggi ' + hm : diff === 1 ? 'ieri ' + hm : two(d.getDate()) + '/' + two(d.getMonth() + 1) + ' ' + hm;
  }
  async function openHistory() {
    if (st.busy) return;
    let list = [];
    try { list = await TAURI.core.invoke('history_list'); } catch (e) { /* vuota */ }
    const p = settingsPanel('Impostazioni › Cronologia');
    p.appendChild(settingsTabs(2));
    const err = el('p', 'perr');
    err.setAttribute('role', 'alert');
    if (!list.length) p.appendChild(el('p', 'hint', 'Nessuna operazione ancora. Qui compariranno le ultime 50 operazioni che hanno creato dei file.'));
    else {
      const box = el('div', 'plist hist');
      list.forEach((v) => {
        const row = el('div', 'hrow'), nm = el('span', 'nm');
        const name = v.file.split('/').pop() + (v.n > 1 ? ' e altri ' + (v.n - 1) : '');
        const made = v.creati.map((c, k) => c.split('/').pop() + (v.esistono[k] ? '' : ' (non c\'è più)')).join(', ');
        nm.append(el('span', null, labelForKey(v.azione)), el('span', 'dir', fmtWhen(v.quando) + ' · ' + name), el('span', 'dir', '→ ' + made));
        nm.title = v.esito;
        row.appendChild(nm);
        const alive = v.creati.filter((_, k) => v.esistono[k]);
        if (alive.length) {
          const show = el('button', 'ghost mini', 'Mostra'), bin = el('button', 'ghost mini', 'Cestino');
          show.type = bin.type = 'button';
          show.setAttribute('aria-label', 'Mostra nella cartella: ' + made);
          bin.setAttribute('aria-label', 'Sposta nel cestino: ' + made);
          show.addEventListener('click', async () => {
            try { await TAURI.core.invoke('show_in_folder', { paths: alive }); } catch (e) { err.textContent = String(e); }
          });
          bin.addEventListener('click', async () => {
            // Primo clic: chiede conferma; secondo clic: sposta nel cestino i file creati (mai l'originale).
            if (bin.textContent !== 'Conferma') { bin.textContent = 'Conferma'; bin.classList.add('danger'); return; }
            try { toast(await TAURI.core.invoke('history_trash', { quando: v.quando }), 'ok'); openHistory(); }
            catch (e) { err.textContent = String(e && e.message ? e.message : e); }
          });
          const hb = el('span', 'hbtns');
          hb.append(show, bin);
          row.appendChild(hb);
        }
        box.appendChild(row);
      });
      p.appendChild(box);
      p.appendChild(el('p', 'hint', '«Cestino» sposta nel cestino solo i file creati da Halo: i file di partenza non si toccano.'));
    }
    const btns = el('div', 'pbtns'), clear = el('button', 'ghost', 'Svuota'), close = el('button', 'go', 'Chiudi');
    clear.type = close.type = 'button';
    clear.style.marginRight = 'auto';
    clear.disabled = !list.length;
    clear.addEventListener('click', async () => {
      if (clear.textContent !== 'Conferma') { clear.textContent = 'Conferma'; return; }
      try { await TAURI.core.invoke('history_clear'); openHistory(); } catch (e) { err.textContent = String(e); }
    });
    close.addEventListener('click', () => closePanel(true));
    btns.append(clear, close);
    p.append(err, btns);
    showPanel(p, close, 'Cronologia: ' + list.length + ' operazioni. «Svuota» toglie solo l\'elenco, i file restano.');
  }

  function settingsPanel(title) {
    closePanel(true);
    const p = el('form', 'panel settings');
    p.noValidate = true;
    p.setAttribute('role', 'dialog');
    p.setAttribute('aria-label', title);
    const head = el('div', 'phead'), pi = el('span', 'picon');
    pi.appendChild(icon(GEAR, 18, 1.8));
    head.append(pi, el('span', 'ptitle', title));
    p.appendChild(head);
    return p;
  }
  function showPanel(p, focus, msg) {
    document.body.appendChild(p);
    st.panel = p;
    placePanel(p);
    if (focus) focus.focus();
    say(msg);
  }
  async function saveShortcuts(list) {
    if (!inApp) settings.scorciatoie = list;
    else {
      try {
        settings = await TAURI.core.invoke('save_settings', { settings: { scorciatoie: list } });
      } catch (e) { return String(e && e.message ? e.message : e); }
    }
    applyShortcuts();
    if (st.level == null && refs.fills.length) renderRing();
    return null;
  }
  const scDesc = (s) => (SC_MODI.find((m) => m[0] === s.modo) || ['', ''])[1] + ' · ' +
    s.tipi.map((x) => (SC_TIPI.find((y) => y[0] === x) || [x, x])[1]).join(', ');

  function openShortcuts() {
    if (st.busy) return;
    const p = settingsPanel('Impostazioni › Scorciatoie');
    p.appendChild(settingsTabs(1));
    const list = settings.scorciatoie || [];
    if (list.length) {
      const box = el('div', 'plist');
      list.forEach((s, k) => {
        const row = el('div', 'screw'), nm = el('span', 'nm'), edit = el('button', 'ghost mini', 'Modifica'), del = el('button', 'ghost mini', 'Elimina');
        nm.append(el('span', null, s.nome), el('span', 'dir', scDesc(s)));
        edit.type = del.type = 'button';
        edit.setAttribute('aria-label', 'Modifica ' + s.nome); del.setAttribute('aria-label', 'Elimina ' + s.nome);
        edit.addEventListener('click', () => openShortcutEdit(k));
        del.addEventListener('click', async () => {
          // Primo clic: chiede conferma; secondo clic: elimina.
          if (del.textContent !== 'Conferma') { del.textContent = 'Conferma'; del.classList.add('danger'); return; }
          const e2 = await saveShortcuts(list.filter((_, j) => j !== k));
          if (e2) { err.textContent = e2; return; }
          openShortcuts();
        });
        row.append(nm, edit, del);
        box.appendChild(row);
      });
      p.appendChild(box);
    }
    const add = el('button', 'ghost pick', 'Aggiungi scorciatoia');
    add.type = 'button';
    add.disabled = list.length >= 12;
    add.addEventListener('click', () => openShortcutEdit(null));
    p.appendChild(add);
    p.append(el('p', 'hint', list.length
      ? 'Le scorciatoie compaiono nello spicchio «Le mie app» dei tipi di file scelti (al massimo ' + SC_MAX_MENU + ' per tipo).'
      : 'Nessuna scorciatoia. Aggiungine una per aprire i file con le tue app o eseguire un comando: comparirà nello spicchio «Le mie app».'));
    const err = el('p', 'perr');
    err.setAttribute('role', 'alert');
    const btns = el('div', 'pbtns'), close = el('button', 'go', 'Chiudi');
    close.type = 'button';
    close.addEventListener('click', () => closePanel(true));
    btns.appendChild(close);
    p.append(err, btns);
    showPanel(p, add.disabled ? close : add, 'Scorciatoie: ' + (list.length ? list.map((s) => s.nome).join(', ') : 'nessuna') + '. Esc per chiudere.');
  }

  let appsCache = null;
  async function openShortcutEdit(index) {
    if (st.busy) return;
    const old = index == null ? null : settings.scorciatoie[index];
    const sc = old ? JSON.parse(JSON.stringify(old)) : { nome: '', tipi: [], modo: 'apri', app: '', comando: '' };
    const p = settingsPanel('Impostazioni › ' + (old ? 'Modifica' : 'Nuova scorciatoia'));
    const err = el('p', 'perr');
    err.setAttribute('role', 'alert');

    const field = (label, value, ph) => {
      const l = el('label', null, label), f = el('span', 'field'), inp = document.createElement('input');
      inp.type = 'text'; inp.autocomplete = 'off'; inp.spellcheck = false; inp.value = value || ''; if (ph) inp.placeholder = ph;
      f.appendChild(inp); l.appendChild(f);
      return { l, inp };
    };
    const nome = field('Nome nel menu', sc.nome, 'Upscayl');
    nome.inp.maxLength = 30;
    p.appendChild(nome.l);

    // App: dall'elenco delle app installate oppure un programma / AppImage scelto a mano.
    const g = el('div', 'pgroup', 'App');
    const chosen = el('p', 'picked', sc.app ? sc.app.split('/').pop() : 'Nessuna app scelta.');
    const row = el('div', 'pbtns left'), fromList = el('button', 'ghost pick', 'Dall\'elenco…'), fromFile = el('button', 'ghost pick', 'Scegli file…');
    fromList.type = fromFile.type = 'button';
    row.append(fromList, fromFile);
    g.append(chosen, row);
    const setApp = (file, label) => {
      sc.app = file; chosen.textContent = label;
      // Upscayl: proposto «Ingrandisci 4×» sulle immagini.
      if (/upscayl/i.test(file + label) && !old) { modo.pick(2); }
    };
    fromFile.addEventListener('click', async () => {
      let r = null;
      try { r = await TAURI.core.invoke('pick_program'); } catch (e) { /* annullato */ }
      if (r) setApp(r, r.split('/').pop());
    });
    let listBox = null;
    fromList.addEventListener('click', async () => {
      if (listBox) { listBox.remove(); listBox = null; return; }
      if (!appsCache) { try { appsCache = await TAURI.core.invoke('list_apps'); } catch (e) { appsCache = []; } }
      listBox = el('div', 'pgroup');
      const f = el('span', 'field'), q = document.createElement('input'), box = el('div', 'plist');
      q.type = 'text'; q.placeholder = 'Cerca un\'app…'; q.setAttribute('aria-label', 'Cerca un\'app');
      f.appendChild(q);
      const rows = appsCache.map((a) => {
        const b = el('button', 'approw', a.nome);
        b.type = 'button';
        b.addEventListener('click', () => { setApp(a.file, a.nome); listBox.remove(); listBox = null; fromList.focus(); placePanel(p); });
        box.appendChild(b);
        return [a.nome.toLowerCase(), b];
      });
      q.addEventListener('input', () => { const v = q.value.trim().toLowerCase(); rows.forEach(([n, b]) => { b.hidden = !!v && !n.includes(v); }); });
      listBox.append(f, box);
      g.after(listBox);
      placePanel(p);
      q.focus();
    });
    p.appendChild(g);

    const gm = el('div', 'pgroup', 'Cosa fa');
    const cmd = field('Comando', sc.comando, 'cp {file} ~/Backup/');
    const cmdHelp = el('p', 'hint', '{file} = il file, {cartella} = la sua cartella, {nome} = il nome senza estensione. Parte nella cartella del file.');
    const modoHelp = el('p', 'hint');
    let modoK = Math.max(0, SC_MODI.findIndex((m) => m[0] === sc.modo));
    const modoChoice = choices('seg', SC_MODI.map((m) => m[1]), null, modoK, 3, (k) => {
      modoK = k; sc.modo = SC_MODI[k][0];
      cmd.l.hidden = cmdHelp.hidden = k !== 1;
      modoHelp.textContent = ['L\'app si apre con il file.', '', 'Con il programma di Upscayl (intelligenza artificiale, come richiesto): crea «nome_4x» accanto all\'immagine. Solo per le immagini.'][k];
      modoHelp.hidden = k === 1;
      if (tipiBoxes) tipiBoxes.forEach(([x, c]) => { c.disabled = k === 2 && x !== 'image'; if (k === 2) c.checked = x === 'image'; });
      if (p.isConnected) placePanel(p);
    });
    const modo = { pick: (k) => modoChoice.box.children[k].click() };
    modoChoice.box.setAttribute('aria-label', 'Cosa fa');
    gm.append(modoChoice.box, modoHelp, cmd.l, cmdHelp);
    p.appendChild(gm);

    const gt = el('div', 'pgroup', 'Per i file');
    const chips = el('div', 'chips');
    var tipiBoxes = SC_TIPI.map(([x, label]) => {
      const l = el('label'), c = document.createElement('input');
      c.type = 'checkbox'; c.checked = sc.tipi.indexOf(x) >= 0;
      l.append(c, document.createTextNode(label));
      chips.appendChild(l);
      return [x, c];
    });
    gt.appendChild(chips);
    p.appendChild(gt);
    modoChoice.box.children[modoK].click();

    const btns = el('div', 'pbtns'), cancel = el('button', 'ghost', 'Annulla'), go = el('button', 'go', 'Salva');
    cancel.type = 'button'; go.type = 'submit';
    btns.append(cancel, go);
    p.append(err, btns);
    cancel.addEventListener('click', () => openShortcuts());
    p.addEventListener('submit', async (e) => {
      e.preventDefault();
      sc.nome = nome.inp.value.trim();
      sc.comando = cmd.inp.value.trim();
      sc.tipi = tipiBoxes.filter(([, c]) => c.checked).map(([x]) => x);
      const others = (settings.scorciatoie || []).filter((_, j) => j !== index);
      if (!sc.nome) { err.textContent = 'Scrivi il nome.'; nome.inp.focus(); return; }
      if (others.some((s) => s.nome.toLowerCase() === sc.nome.toLowerCase())) { err.textContent = 'C\'è già una scorciatoia con questo nome.'; nome.inp.focus(); return; }
      if (sc.modo !== 'comando' && !sc.app) { err.textContent = 'Scegli l\'app.'; fromList.focus(); return; }
      if (sc.modo === 'comando' && !sc.comando) { err.textContent = 'Scrivi il comando.'; cmd.inp.focus(); return; }
      if (!sc.tipi.length) { err.textContent = 'Scegli almeno un tipo di file.'; return; }
      const list = (settings.scorciatoie || []).slice();
      if (index == null) list.push(sc); else list[index] = sc;
      go.disabled = cancel.disabled = true;
      const e2 = await saveShortcuts(list);
      go.disabled = cancel.disabled = false;
      if (e2) { err.textContent = e2; return; }
      openShortcuts();
      toast('Scorciatoia salvata: «' + sc.nome + '».', 'ok');
    });
    showPanel(p, nome.inp, (old ? 'Modifica di ' + old.nome : 'Nuova scorciatoia') + ': nome, app, cosa fa e tipi di file. Esc per chiudere.');
  }

  function closeMenu() {
    if (inApp) {
      try { TAURI.window.getCurrentWindow().close(); } catch (e) { window.close(); }
      return;
    }
    menu.classList.add('hidden');
    say('Menu chiuso. Clic in un punto qualsiasi per riaprirlo.');
  }

  function place(x, y) {
    const W = window.innerWidth, H = window.innerHeight, off = menuInset();
    const left = Math.max(12 - off, Math.min(x - MENU / 2, W - MENU - 12 + off));
    const top = Math.max(12 - off, Math.min(y - MENU / 2, H - MENU - 60 + off));
    menu.style.left = left + 'px';
    menu.style.top = top + 'px';
    st.placed = true;
  }

  function show(x, y) {
    place(x, y);
    menu.classList.remove('hidden');
    renderRing();
  }

  document.addEventListener('keydown', (e) => {
    // Con il pannello aperto i tasti servono ai campi; Esc lo chiude e torna al menu.
    if (st.panel) {
      if (e.key === 'Escape' && !st.busy) { e.preventDefault(); closePanel(true); }
      return;
    }
    if (menu.classList.contains('hidden') || st.busy) return;
    // Cerca scrivendo: lettere e numeri filtrano le azioni; frecce e Invio scelgono tra i risultati.
    if (e.ctrlKey && e.key === ',') { e.preventDefault(); openSettings(); return; }
    const plain = !e.ctrlKey && !e.altKey && !e.metaKey;
    if (plain && e.key.length === 1 && (/[\p{L}\p{N}]/u.test(e.key) || (e.key === ' ' && st.query))) {
      e.preventDefault(); st.query += e.key; st.searchSel = 0; renderSearch(); return;
    }
    if (st.query) {
      const res = results();
      if (e.key === 'Backspace') { e.preventDefault(); st.query = st.query.slice(0, -1); renderSearch(); return; }
      if (e.key === 'Escape') { e.preventDefault(); clearSearch(); return; }
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        if (res.length) st.searchSel = (st.searchSel + (e.key === 'ArrowDown' ? 1 : res.length - 1)) % res.length;
        renderSearch(); return;
      }
      if (e.key === 'Enter') { e.preventDefault(); if (res[st.searchSel]) chooseResult(res[st.searchSel]); return; }
    }
    const n = items().length;
    if (e.key === 'Escape' || e.key === 'Backspace') {
      e.preventDefault();
      if (st.level != null && !T().mixed) back(); else if (e.key === 'Escape') closeMenu();
      return;
    }
    let next = null;
    if (e.key === 'ArrowRight' || e.key === 'ArrowDown') next = (st.focus + 1) % n;
    else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') next = (st.focus - 1 + n) % n;
    else if (e.key === 'Home') next = 0;
    else if (e.key === 'End') next = n - 1;
    else if (e.key === 'Tab') { st.kb = true; return; }
    if (next == null) return;
    e.preventDefault();
    st.kb = true; st.focus = next; paint(); focusWedge(next);
  });
  document.addEventListener('mousemove', () => { if (st.kb) { st.kb = false; paint(); } });

  // ---------- impostazioni: tieni premuto il cerchio centrale per 3 secondi ----------
  const HOLD_MS = 1500;
  const holdSvg = svg(168, 168, { class: 'holdring', viewBox: '0 0 168 168' });
  const holdArc = document.createElementNS(NS, 'circle');
  for (const [k, v] of Object.entries({ cx: 84, cy: 84, r: 81, fill: 'none', stroke: 'var(--accent)', 'stroke-width': 5,
    'stroke-linecap': 'round', 'stroke-dasharray': String(2 * Math.PI * 81), transform: 'rotate(-90 84 84)' })) holdArc.setAttribute(k, v);
  holdSvg.appendChild(holdArc);
  // Gli elementi SVG non hanno la proprietà «hidden»: si usa l'attributo.
  const showHold = (v) => { if (v) holdSvg.removeAttribute('hidden'); else holdSvg.setAttribute('hidden', ''); };
  showHold(false);
  menu.appendChild(holdSvg);
  // Durante la pressione il contenuto del centro (file scelto) lascia il posto all'icona delle impostazioni.
  const holdGear = el('div', 'holdgear');
  holdGear.setAttribute('aria-hidden', 'true');
  const holdIcon = el('span', 'hgicon');
  holdIcon.appendChild(icon(GEAR, 34, 1.7));
  holdGear.append(holdIcon, el('span', 'hglabel', 'Impostazioni'));
  menu.appendChild(holdGear);
  const setHolding = (on) => {
    holdGear.classList.toggle('on', on);
    const c = slot.querySelector('.center');
    if (c) c.classList.toggle('holding', on);
  };
  let hold = null, holdDone = false;
  function stopHold() {
    if (!hold) return;
    cancelAnimationFrame(hold.raf);
    hold = null;
    showHold(false);
    setHolding(false);
    holdIcon.style.transform = '';
  }
  slot.addEventListener('pointerdown', (e) => {
    if (e.button !== 0 || st.busy || hold) return;
    holdDone = false;
    hold = { t0: performance.now(), raf: 0 };
    const len = 2 * Math.PI * 81;
    const step = (now) => {
      if (!hold) return;
      const p = Math.min(1, (now - hold.t0) / HOLD_MS);
      // Il riempimento compare dopo un attimo, così un clic normale non lo fa lampeggiare.
      const visible = now - hold.t0 >= 150;
      showHold(visible);
      setHolding(visible);
      // L'ingranaggio gira mentre l'anello si riempie.
      holdIcon.style.transform = 'rotate(' + Math.round(p * 180) + 'deg)';
      holdArc.setAttribute('stroke-dashoffset', String(len * (1 - p)));
      if (p >= 1) { stopHold(); holdDone = true; openSettings(); return; }
      hold.raf = requestAnimationFrame(step);
    };
    hold.raf = requestAnimationFrame(step);
  });
  ['pointerup', 'pointercancel', 'pointerleave'].forEach((ev) => slot.addEventListener(ev, stopHold));
  // Dopo la pressione lunga, il clic che segue non deve fare «Indietro» o «Ripeti».
  slot.addEventListener('click', (e) => { if (holdDone) { holdDone = false; e.stopPropagation(); e.preventDefault(); } }, true);

  // ---------- chiusura dopo un risultato ----------
  // Il menu resta aperto 5 secondi; se nel frattempo lo usi (clic, tasti) resta aperto.
  let closeTimer = 0;
  // Senza `ms` usa il tempo scelto nelle impostazioni (3, 5, 10 secondi o mai).
  function closeLater(ms) {
    clearTimeout(closeTimer);
    const pref = (settings.aspetto || {}).chiusura || '5';
    if (!ms && pref === 'mai') return;
    closeTimer = setTimeout(closeMenu, ms || Number(pref) * 1000);
  }
  const keepOpen = () => { clearTimeout(closeTimer); closeTimer = 0; };
  menu.addEventListener('pointerdown', keepOpen, true);
  document.addEventListener('keydown', keepOpen, true);
  document.addEventListener('pointerdown', (e) => { if (e.target.closest && e.target.closest('.panel')) keepOpen(); }, true);
  $('backdrop').addEventListener('click', (e) => {
    if (inApp) { closeMenu(); return; }
    if (menu.classList.contains('hidden')) show(e.clientX, e.clientY); else closeMenu();
  });

  // ---------- avvio ----------
  function buildTabs() {
    const tabs = $('tabs');
    TYPES.forEach((t, i) => {
      const b = el('button', null, t.tab);
      b.type = 'button';
      b.setAttribute('aria-pressed', i === st.type ? 'true' : 'false');
      b.addEventListener('click', () => {
        st.type = i; st.level = null; st.focus = 0; st.chosen = null; st.result = null;
        st.anim = 'rm-in'; st.animCenter = 'rm-in';
        tabs.querySelectorAll('button').forEach((x, j) => x.setAttribute('aria-pressed', j === i ? 'true' : 'false'));
        toast('');
        if (menu.classList.contains('hidden')) show(window.innerWidth / 2, (window.innerHeight - 60) / 2);
        else renderRing();
        say(t.tab + ': ' + t.acts.map((a) => a[0]).join(', ') + '.');
      });
      tabs.appendChild(b);
    });
  }

  // Aggiunge file arrivati dopo l'apertura e passa al menu per più file.
  function addFiles(list) {
    const before = st.files.length;
    list.forEach((f) => { if (!st.files.some((x) => x.path === f.path)) st.files.push(f); });
    if (st.files.length === before || st.busy || st.panel) return;
    for (let k = TYPES.length - 1; k >= 0; k--) if (TYPES[k].multi) TYPES.splice(k, 1);
    TYPES.push(multiType(st.files));
    st.forms.set('multi/insieme/rinomina-in-serie', 'nome-serie-multi');
    st.file = st.files[0]; st.type = TYPES.length - 1;
    st.level = T().mixed ? 0 : null; st.focus = 0; st.chosen = null; st.result = null;
    st.anim = 'rm-in'; st.animCenter = 'rm-in';
    renderRing();
    say(st.files.length + ' file scelti.');
  }

  async function startApp() {
    try { settings = await TAURI.core.invoke('get_settings'); } catch (e) { /* aspetto predefinito */ }
    applySettings(settings.aspetto);
    applyShortcuts();
    let file = null;
    try { st.files = await TAURI.core.invoke('target_files'); } catch (e) { st.files = []; }
    file = st.files[0] || null;
    try { st.supported = new Set(await TAURI.core.invoke('supported_actions')); } catch (e) { st.supported = new Set(); }
    try { st.forms = new Map(await TAURI.core.invoke('form_actions')); } catch (e) { st.forms = new Map(); }

    // Il menu compare dove si trova il puntatore: aspettiamo il primo movimento (max 250 ms),
    // altrimenti lo mettiamo al centro dello schermo.
    // Alt+clic: l'estensione di GNOME passa la posizione del puntatore (--pos), così il menu
    // compare lì anche se il mouse resta fermo. Altrimenti si aspetta il primo movimento.
    let given = null;
    try { given = await TAURI.core.invoke('start_position'); } catch (e) { /* nessuna posizione */ }
    const where = given ? { x: given[0], y: given[1] } : await new Promise((resolve) => {
      const done = (e) => { window.removeEventListener('mousemove', done); window.removeEventListener('mouseover', done); resolve({ x: e.clientX, y: e.clientY }); };
      window.addEventListener('mousemove', done);
      window.addEventListener('mouseover', done);
      setTimeout(() => resolve(null), 250);
    });
    const x = where ? where.x : window.innerWidth / 2, y = where ? where.y : window.innerHeight / 2;
    // Se la finestra finisce di massimizzarsi dopo l'apertura, il menu torna nel punto giusto.
    if (given) {
      const replace = () => { if (!menu.classList.contains('hidden') && st.level == null && !st.panel) place(x, y); };
      window.addEventListener('resize', replace);
      setTimeout(() => window.removeEventListener('resize', replace), 1500);
    }

    if (!file) {
      show(x, y);
      toast('Nessun file ricevuto. Avvia con: halo /percorso/del/file', 'err');
      return;
    }
    // Windows: con più file scelti in Esplora file ne arrivano altri dalle istanze successive.
    if (TAURI.event) TAURI.event.listen('altri-file', (e) => addFiles(e.payload || [])).catch(() => {});
    if (isMulti()) {
      // Più file: menu dedicato; con tipi diversi si apre direttamente su «Insieme».
      TYPES.push(multiType(st.files));
      st.forms.set('multi/insieme/rinomina-in-serie', 'nome-serie-multi');
      st.file = file; st.type = TYPES.length - 1;
      if (T().mixed) st.level = 0;
      show(x, y);
      say(st.files.length + ' file scelti. Frecce per scegliere, Invio per confermare, Esc per chiudere.');
      return;
    }
    const idx = TYPES.findIndex((t) => t.kind === file.kind);
    if (idx < 0) {
      show(x, y);
      toast('«' + file.name + '»: tipo di file non ancora supportato.', 'err');
      setTimeout(closeMenu, 3000);
      return;
    }
    st.file = file; st.type = idx;
    show(x, y);
    say('Azioni per ' + file.name + '. Frecce per scegliere, Invio per confermare, Esc per chiudere. Tieni premuto il centro per 3 secondi, o Ctrl+virgola, per le impostazioni.');
  }

  if (inApp) {
    startApp();
  } else {
    document.body.classList.add('demo');
    buildTabs();
    show(window.innerWidth / 2, (window.innerHeight - 60) / 2);
  }
})();

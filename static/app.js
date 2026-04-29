const statusEl    = document.getElementById('status');
const gallery     = document.getElementById('gallery');
const programEl   = document.getElementById('program');
const renderSingleBtn         = document.getElementById('render-single-btn');
const renderSinglePreviewBtn  = document.getElementById('render-single-preview-btn');
const renderSingleLargeBtn    = document.getElementById('render-single-large-btn');
const renderBtn               = document.getElementById('render-btn');
const renderPreviewBtn        = document.getElementById('render-preview-btn');
const renderLargeBtn          = document.getElementById('render-large-btn');
const renderCompoundBtn       = document.getElementById('render-compound-btn');
const renderCompoundPreviewBtn = document.getElementById('render-compound-preview-btn');
const renderCompoundLargeBtn  = document.getElementById('render-compound-large-btn');
const randomBtn          = document.getElementById('random-btn');
const randomSimpleBtn    = document.getElementById('random-simple-btn');
const randomComplexBtn   = document.getElementById('random-complex-btn');
const random20Btn        = document.getElementById('random20-btn');
const random20SimpleBtn  = document.getElementById('random20-simple-btn');
const random20ComplexBtn = document.getElementById('random20-complex-btn');
const galleryBtn         = document.getElementById('gallery-btn');
const savedBtn           = document.getElementById('saved-btn');
const errorMsg    = document.getElementById('error-msg');
const compareBar  = document.getElementById('compare-bar');
const compareImgs = document.getElementById('compare-images');
const compareClear = document.getElementById('compare-clear');
const zoomModal   = document.getElementById('zoom-modal');
const zoomCanvas  = document.getElementById('zoom-canvas');
const zoomStatus  = document.getElementById('zoom-status');

// ── Zoom modal ─────────────────────────────────────────────────────────────────

// AbortController for the in-flight full-res render, if any. Cancelled on
// modal close or when a new zoom starts, so we don't waste render time on
// an image the user can no longer see.
let zoomAbort = null;

function setZoomStatus(text) {
  zoomStatus.textContent = text || '';
  zoomStatus.classList.toggle('show', !!text);
}

function closeZoom() {
  if (zoomAbort) { zoomAbort.abort(); zoomAbort = null; }
  zoomModal.classList.remove('open');
  setZoomStatus('');
}

function showZoom(srcCanvas, programText) {
  // Supersede any previous full-res upgrade in flight.
  if (zoomAbort) { zoomAbort.abort(); zoomAbort = null; }

  zoomCanvas.width  = srcCanvas.width;
  zoomCanvas.height = srcCanvas.height;
  zoomCanvas.getContext('2d').drawImage(srcCanvas, 0, 0);
  zoomModal.classList.add('open');
  setZoomStatus('');

  // Only gallery cards pass programText through — they're the ones whose
  // thumbnails have been downsampled to GALLERY_MAX_DIM and benefit from
  // an on-demand native-resolution render.
  if (!programText) return;

  const ctrl = new AbortController();
  zoomAbort = ctrl;
  setZoomStatus('loading full resolution…');

  fetchSingleRender(programText, ctrl.signal)
    .then(payload => {
      if (zoomAbort !== ctrl || !zoomModal.classList.contains('open')) return;
      // Don't swap if the native render isn't actually larger.
      if (payload.width <= srcCanvas.width && payload.height <= srcCanvas.height) {
        setZoomStatus('');
        return;
      }
      const img = new Image();
      img.onload = () => {
        if (zoomAbort !== ctrl || !zoomModal.classList.contains('open')) return;
        zoomCanvas.width = payload.width;
        zoomCanvas.height = payload.height;
        zoomCanvas.getContext('2d').drawImage(img, 0, 0);
        setZoomStatus('');
        zoomAbort = null;
      };
      img.src = 'data:image/webp;base64,' + payload.webp_b64;
    })
    .catch(err => {
      if (err.name === 'AbortError' || zoomAbort !== ctrl) return;
      setZoomStatus('full-res unavailable');
    });
}

// Async generator over an ND-JSON response body. Breaking out of the
// consuming `for await` loop runs the finally block and cancels the
// reader, so callers can stop early without leaking the stream.
async function* readNdjson(res) {
  const reader = res.body.getReader();
  const decoder = new TextDecoder();
  let buf = '';
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buf += decoder.decode(value, { stream: true });
      let nl;
      while ((nl = buf.indexOf('\n')) !== -1) {
        const line = buf.slice(0, nl).trim();
        buf = buf.slice(nl + 1);
        if (!line) continue;
        try { yield JSON.parse(line); } catch { /* skip malformed line */ }
      }
    }
  } finally {
    reader.cancel().catch(() => {});
  }
}

async function fetchSingleRender(programText, signal) {
  const res = await fetch('/api/render', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ program_text: programText, mode: 'single', size: 0 }),
    signal,
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  for await (const item of readNdjson(res)) {
    if (item.type === 'original') return item.image;
  }
  throw new Error('no original in single-render stream');
}

zoomModal.addEventListener('click', closeZoom);

document.addEventListener('keydown', e => {
  if (e.key === 'Escape') closeZoom();
});

// ── Saved store (localStorage) ────────────────────────────────────────────────

// Persisted entries each look like:
//   { id, savedAt, label, programText, jxl_size }
// We deliberately don't cache the rendered webp here — full-res webp can run
// hundreds of KB per program, so a handful of saves used to blow the 5MB
// localStorage quota. Storing only `programText` (a few hundred bytes) lets
// the saved view fit thousands of entries; opening the view re-renders each
// program against `/api/render` on demand.
// Dedup is by `programText`, so clicking ★ on two cards that show the same
// program toggles a single saved entry.
const SAVED_KEY = 'artxl.saved.v2';
let savedIdCounter = 0;

function loadSaved() {
  try {
    const raw = localStorage.getItem(SAVED_KEY);
    if (!raw) return [];
    const arr = JSON.parse(raw);
    return Array.isArray(arr) ? arr : [];
  } catch {
    return [];
  }
}

function persistSaved(arr) {
  try {
    localStorage.setItem(SAVED_KEY, JSON.stringify(arr));
    return true;
  } catch (e) {
    errorMsg.textContent = e && e.name === 'QuotaExceededError'
      ? 'Save failed: localStorage is full. Remove some saved images first.'
      : 'Save failed: ' + (e && e.message ? e.message : 'unknown error');
    return false;
  }
}

function findSaved(programText) {
  if (!programText) return undefined;
  return loadSaved().find(e => e.programText === programText);
}

function addSaved({ label, programText, jxl_size }) {
  const arr = loadSaved();
  if (arr.some(e => e.programText === programText)) return null;
  const entry = {
    id: ++savedIdCounter,
    savedAt: Date.now(),
    label,
    programText,
    jxl_size: jxl_size ?? 0,
  };
  arr.push(entry);
  if (!persistSaved(arr)) return null;
  return entry;
}

function removeSaved(id) {
  const arr = loadSaved().filter(e => e.id !== id);
  persistSaved(arr);
}

// Make sure new ids don't collide with persisted ones across reloads.
function initSavedIdCounter() {
  const arr = loadSaved();
  savedIdCounter = arr.reduce((m, e) => Math.max(m, e.id || 0), 0);
}

// Drop saves from the previous schema (which embedded full webp_b64 payloads
// and blew the 5MB localStorage quota). Best-effort; ignored on failure.
try { localStorage.removeItem('artxl.saved.v1'); } catch {}

// ── Comparison state ──────────────────────────────────────────────────────────

// Maps a unique id → { srcCanvas, el (the .cmp-item div) }
const pinned = new Map();
let pinId = 0;

function togglePin(srcCanvas, label) {
  // already pinned? → unpin
  for (const [id, { srcCanvas: c }] of pinned) {
    if (c === srcCanvas) { unpin(id); return; }
  }
  // not pinned → pin
  const id = pinId++;

  // Copy pixels into a new canvas for the bar
  const c = document.createElement('canvas');
  c.width  = srcCanvas.width;
  c.height = srcCanvas.height;
  c.getContext('2d').drawImage(srcCanvas, 0, 0);
  c._fullResProgram = srcCanvas._fullResProgram || null;
  c.title = 'Click to zoom';
  c.addEventListener('click', () => showZoom(c, c._fullResProgram));

  const lbl = document.createElement('div');
  lbl.className = 'cmp-label';
  lbl.innerHTML = `<span>${label}</span>`;

  const rmBtn = document.createElement('button');
  rmBtn.className = 'cmp-remove';
  rmBtn.textContent = '✕';
  rmBtn.addEventListener('click', () => unpin(id));
  lbl.appendChild(rmBtn);

  const item = document.createElement('div');
  item.className = 'cmp-item';
  item.appendChild(c);
  item.appendChild(lbl);
  compareImgs.appendChild(item);

  pinned.set(id, { srcCanvas, el: item });
  srcCanvas.classList.add('pinned');
  compareBar.style.display = 'block';
  requestAnimationFrame(syncBarPadding);
}

function unpin(id) {
  const { srcCanvas, el } = pinned.get(id);
  srcCanvas.classList.remove('pinned');
  el.remove();
  pinned.delete(id);
  if (pinned.size === 0) {
    compareBar.style.display = 'none';
    setBarPadding('');
  } else {
    requestAnimationFrame(syncBarPadding);
  }
}

function clearAllPins() {
  for (const [id] of [...pinned]) unpin(id);
}

compareClear.addEventListener('click', clearAllPins);

// In wide layout the body doesn't scroll — the panes do — so the
// pinned-comparison-bar bottom padding has to be applied to the panes
// too. Setting it on body as well is harmless in narrow mode (where
// body is the scroller) and a no-op in wide mode.
function setBarPadding(value) {
  document.body.style.paddingBottom = value;
  const left = document.getElementById('left-pane');
  const right = document.getElementById('right-pane');
  if (left) left.style.paddingBottom = value;
  if (right) right.style.paddingBottom = value;
}

function syncBarPadding() {
  setBarPadding(compareBar.offsetHeight + 'px');
}

// ── Streaming fetch ───────────────────────────────────────────────────────────

async function streamFrom(url, method, body, size = 0) {
  const fullUrl = size ? url + (url.includes('?') ? '&' : '?') + `size=${size}` : url;
  const opts = { method };
  if (body) {
    opts.headers = { 'Content-Type': 'application/json' };
    opts.body = JSON.stringify(size ? { ...body, size } : body);
  }
  const res = await fetch(fullUrl, opts);
  if (!res.ok) {
    const msg = await res.text();
    throw new Error(msg || `HTTP ${res.status}`);
  }

  let mutationCount = 0;
  let rendered = 0;
  let simpleSectionAdded = false;
  let compoundSectionAdded = false;

  for await (const item of readNdjson(res)) {
    if (item.type === 'batch_image') {
      renderCard(gallery, `Random ${item.index + 1}`, item.image, false, item.program_text);
      rendered++;
      statusEl.textContent = `Generated ${rendered} / ${item.total}…`;
      if (rendered === item.total) statusEl.textContent = `Generated ${rendered} random images.`;
    } else if (item.type === 'original') {
      mutationCount = item.mutation_count;
      programEl.value = item.program_text;
      renderCard(gallery, 'Original', item.image, true, item.program_text);
      rendered++;
      statusEl.textContent = `Rendering… (${rendered} / ${mutationCount + 1})`;
    } else if (item.type === 'mutation') {
      if (!item.compound && !simpleSectionAdded) {
        addSectionHeader(gallery, 'Simple mutations');
        simpleSectionAdded = true;
      }
      if (item.compound && !compoundSectionAdded) {
        addSectionHeader(gallery, 'Compound mutations');
        compoundSectionAdded = true;
      }
      renderCard(gallery, item.label, item.image, false, item.program_text, item.warning);
      rendered++;
      statusEl.textContent = `Rendering… (${rendered} / ${mutationCount + 1})`;
    } else if (item.type === 'gallery_image') {
      renderCard(gallery, item.name, item.image, false, item.program_text, null, true);
      rendered++;
      statusEl.textContent = `Loaded ${rendered} / ${item.total} gallery image(s).`;
    } else if (item.type === 'done') {
      statusEl.textContent = `Rendered ${rendered} images.`;
    }
  }
}

// ── Data loading ──────────────────────────────────────────────────────────────

async function main() {
  gallery.innerHTML = '';
  clearAllPins();
  errorMsg.textContent = '';
  statusEl.textContent = 'Generating…';
  try {
    const zcode = new URLSearchParams(location.search).get('zcode');
    if (zcode && zcodeSupported) {
      try {
        const programText = await decodeZcode(zcode);
        await streamFrom('/api/render', 'POST',
          { program_text: programText, mode: 'single' }, 0);
        return;
      } catch (e) {
        console.error('bad zcode, falling back to /api/generate', e);
        errorMsg.textContent = 'Share link is invalid — showing a random program instead.';
      }
    }
    await streamFrom('/api/generate', 'GET', null, 0);
  } catch (e) {
    statusEl.textContent = `Error: ${e.message}`;
  }
}

const allBtns = [
  renderSingleBtn, renderSinglePreviewBtn, renderSingleLargeBtn,
  renderBtn, renderPreviewBtn, renderLargeBtn,
  renderCompoundBtn, renderCompoundPreviewBtn, renderCompoundLargeBtn,
  randomBtn, randomSimpleBtn, randomComplexBtn,
  random20Btn, random20SimpleBtn, random20ComplexBtn,
  galleryBtn, savedBtn,
];

let currentMode = 'normal'; // 'normal' | 'gallery' | 'saved'

function resetModeToggles() {
  gallery.classList.remove('gallery-mode');
  if (savedAbort) { savedAbort.abort(); savedAbort = null; }
  currentMode = 'normal';
  galleryBtn.textContent = 'Gallery';
  updateSavedBtnLabel();
}

function withBusy(btn, label, fn) {
  const orig = btn.textContent;
  btn.textContent = label;
  allBtns.forEach(b => b.disabled = true);
  errorMsg.textContent = '';
  gallery.innerHTML = '';
  clearAllPins();
  if (btn !== galleryBtn && btn !== savedBtn) resetModeToggles();
  fn().catch(e => { errorMsg.textContent = `Error: ${e.message}`; })
      .finally(() => { allBtns.forEach(b => b.disabled = false); btn.textContent = orig; });
}

function bindSizes(btn, previewBtn, largeBtn, busy, url, method, body) {
  btn.addEventListener('click',        () => withBusy(btn,        busy, () => streamFrom(url, method, body(), 0)));
  previewBtn.addEventListener('click', () => withBusy(previewBtn, '…', () => streamFrom(url, method, body(), 320)));
  largeBtn.addEventListener('click',   () => withBusy(largeBtn,   '…', () => streamFrom(url, method, body(), 2048)));
}

// Random-program generators don't take a render-size knob (rendered at the
// program's native 1024×1024); instead the side buttons set tree complexity.
// 0 = simple (smaller tree), 2 = complex (deeper tree).
function bindComplexity(btn, simpleBtn, complexBtn, busy, url) {
  const go = (b, complexity) => withBusy(b, busy,
    () => streamFrom(`${url}?complexity=${complexity}`, 'GET', null, 0));
  btn.addEventListener('click',        () => go(btn, 1));
  simpleBtn.addEventListener('click',  () => go(simpleBtn, 0));
  complexBtn.addEventListener('click', () => go(complexBtn, 2));
}

bindComplexity(random20Btn, random20SimpleBtn, random20ComplexBtn,
  'Generating…', '/api/random/batch');

bindComplexity(randomBtn, randomSimpleBtn, randomComplexBtn,
  'Randomizing…', '/api/random');

bindSizes(renderSingleBtn, renderSinglePreviewBtn, renderSingleLargeBtn,
  'Rendering…', '/api/render', 'POST', () => ({ program_text: programEl.value, mode: 'single' }));

bindSizes(renderBtn, renderPreviewBtn, renderLargeBtn,
  'Rendering…', '/api/render', 'POST', () => ({ program_text: programEl.value, mode: 'mutations' }));

bindSizes(renderCompoundBtn, renderCompoundPreviewBtn, renderCompoundLargeBtn,
  'Rendering…', '/api/render', 'POST', () => ({ program_text: programEl.value, mode: 'compound20' }));

function openGallery() {
  withBusy(galleryBtn, 'Loading…', async () => {
    gallery.classList.add('gallery-mode');
    currentMode = 'gallery';
    galleryBtn.textContent = 'Close gallery';
    addGalleryCredit(gallery);
    await streamFrom('/api/gallery', 'GET', null, 0);
  });
}

// In-flight saved-view renders, cancelled when the view is closed or the
// user jumps into another mode mid-load.
let savedAbort = null;

function openSaved() {
  withBusy(savedBtn, 'Loading…', async () => {
    gallery.classList.add('gallery-mode');
    currentMode = 'saved';
    savedBtn.textContent = 'Close saved';
    await renderSavedView();
  });
}

async function renderSavedView() {
  const arr = loadSaved().slice().sort((a, b) => b.savedAt - a.savedAt);
  if (arr.length === 0) {
    renderSavedEmptyHint();
    statusEl.textContent = 'No saved images yet.';
    return;
  }

  if (savedAbort) savedAbort.abort();
  const ctrl = new AbortController();
  savedAbort = ctrl;

  // Pass 1: drop a placeholder card in saved order so the grid is laid
  // out immediately and individual renders fill in as they complete.
  const slots = arr.map(e => {
    const ph = document.createElement('div');
    ph.className = 'card saved-loading';
    const info = document.createElement('div');
    info.className = 'info';
    const lbl = document.createElement('span');
    lbl.className = 'label';
    lbl.textContent = e.label;
    info.appendChild(lbl);
    const note = document.createElement('div');
    note.className = 'saved-note';
    note.textContent = 'rendering…';
    info.appendChild(note);
    ph.appendChild(info);
    gallery.appendChild(ph);
    return ph;
  });

  let done = 0;
  statusEl.textContent = `Loaded 0 / ${arr.length} saved image(s).`;
  await Promise.all(arr.map(async (e, idx) => {
    if (ctrl.signal.aborted) return;
    try {
      const payload = await fetchSingleRender(e.programText, ctrl.signal);
      payload.jxl_size = e.jxl_size ?? 0;
      if (ctrl.signal.aborted) return;
      const tmp = document.createElement('div');
      renderCard(tmp, e.label, payload, false, e.programText);
      const card = tmp.firstElementChild;
      if (slots[idx].parentNode === gallery) slots[idx].replaceWith(card);
    } catch (err) {
      if (err.name === 'AbortError' || ctrl.signal.aborted) return;
      const note = slots[idx].querySelector('.saved-note');
      if (note) {
        note.textContent = 'render failed';
        note.classList.add('failed');
      }
    } finally {
      if (!ctrl.signal.aborted) {
        done++;
        statusEl.textContent = `Loaded ${done} / ${arr.length} saved image(s).`;
      }
    }
  }));

  if (savedAbort === ctrl) savedAbort = null;
}

function renderSavedEmptyHint() {
  const el = document.createElement('div');
  el.className = 'gallery-empty';
  el.textContent = 'No saved images yet — click ☆ on any card to save it.';
  gallery.appendChild(el);
}

function updateSavedBtnLabel() {
  const n = loadSaved().length;
  if (currentMode === 'saved') return; // 'Close saved' takes precedence
  savedBtn.textContent = n > 0 ? `Saved (${n})` : 'Saved';
}

function refreshAllSaveButtons() {
  for (const b of gallery.querySelectorAll('.dl-btn')) {
    if (typeof b._refreshSaved === 'function') b._refreshSaved();
  }
}

function addGalleryCredit(container) {
  const el = document.createElement('div');
  el.className = 'gallery-credit';
  el.innerHTML =
    'Programs sourced from the <a href="https://discord.com/invite/jpeg-xl-794206087879852103" ' +
    'target="_blank" rel="noopener noreferrer">#jxl-art channel on the JPEG XL Discord</a> ' +
    'and <a href="https://jpegxl.info/art/" target="_blank" rel="noopener noreferrer">jpegxl.info/art/</a>.';
  container.appendChild(el);
}

galleryBtn.addEventListener('click', () => {
  if (currentMode === 'gallery') {
    resetModeToggles();
    main();
  } else {
    openGallery();
  }
});

savedBtn.addEventListener('click', () => {
  if (currentMode === 'saved') {
    resetModeToggles();
    main();
  } else {
    openSaved();
  }
});

// ── Card rendering ────────────────────────────────────────────────────────────

function renderCard(container, label, payload, isOriginal, programText, warning, hideLabel) {
  const card = document.createElement('div');
  card.className = 'card';

  const canvas = document.createElement('canvas');
  canvas.width  = payload.width;
  canvas.height = payload.height;
  const ctx = canvas.getContext('2d');
  const img = new Image();
  img.onload = () => ctx.drawImage(img, 0, 0);
  img.src = 'data:image/webp;base64,' + payload.webp_b64;

  // Gallery thumbnails are downsampled server-side, so on zoom we kick off
  // a native-resolution render in the background. `hideLabel` is the
  // gallery-only flag — mutation / randomize cards keep the simple zoom.
  // Stash the program text on the canvas so pinned copies in the compare
  // bar can also trigger the upgrade.
  canvas._fullResProgram = hideLabel ? programText : null;
  canvas.title = 'Click to zoom';
  canvas.addEventListener('click', () => showZoom(canvas, canvas._fullResProgram));

  const info = document.createElement('div');
  info.className = 'info';

  if (!hideLabel) {
    const lbl = document.createElement('span');
    lbl.className = 'label';
    lbl.textContent = label;
    if (isOriginal) {
      const badge = document.createElement('span');
      badge.className = 'original-badge';
      badge.textContent = 'original';
      lbl.appendChild(badge);
    }
    info.appendChild(lbl);
  }

  // Two rows: row 1 holds the (wider) downloads with the JXL size at its
  // end, row 2 holds the secondary icon-only actions. Splitting them keeps
  // the size pinned to row 1 and gives both rows a similar visual weight,
  // rather than letting flex-wrap shuffle items unpredictably.
  const dlRow = document.createElement('div');
  dlRow.className = 'dl-row';

  const pngBtn = makeBtn('↓ PNG');
  pngBtn.addEventListener('click', () => downloadPng(canvas, label));
  dlRow.appendChild(pngBtn);

  // Server pre-computes jxl_size in the payload (it already has the
  // JXL bytes from the render roundtrip), so we show it immediately
  // without a second subprocess round-trip. 0 means the encoder
  // couldn't produce anything — hide the JXL button in that case.
  const jxlSizeValue = payload.jxl_size ?? 0;
  if (jxlSizeValue > 0) {
    const jxlBtn = makeBtn('↓ JXL');
    const jxlSize = document.createElement('span');
    jxlSize.className = 'jxl-size';
    jxlSize.textContent = fmtBytes(jxlSizeValue);
    jxlBtn.addEventListener('click', async () => {
      jxlBtn.disabled = true;
      jxlBtn.textContent = '…';
      try {
        await downloadJxl(programText ?? programEl.value, label);
      } catch (e) {
        alert('JXL download failed: ' + e.message);
      } finally {
        jxlBtn.disabled = false;
        jxlBtn.textContent = '↓ JXL';
      }
    });
    dlRow.appendChild(jxlBtn);
    dlRow.appendChild(jxlSize);
  }

  const actionRow = document.createElement('div');
  actionRow.className = 'dl-row';

  const cmpBtn = makeBtn('⊞');
  cmpBtn.title = 'Pin to comparison bar';
  cmpBtn.addEventListener('click', () => togglePin(canvas, label));
  actionRow.appendChild(cmpBtn);

  if (programText) {
    const saveBtn = makeBtn('');
    const skin = () => {
      const saved = !!findSaved(programText);
      saveBtn.textContent = saved ? '★' : '☆';
      saveBtn.classList.toggle('saved', saved);
      saveBtn.title = saved ? 'Remove from saved' : 'Save this image';
    };
    skin();
    saveBtn.addEventListener('click', () => {
      const existing = findSaved(programText);
      if (existing) {
        removeSaved(existing.id);
      } else if (!addSaved({ label, programText, jxl_size: payload.jxl_size })) {
        return; // quota / persistence error already surfaced via errorMsg
      }
      // Refresh every save button on screen so cards showing the same
      // program toggle in lockstep, and update the top-right counter.
      refreshAllSaveButtons();
      updateSavedBtnLabel();
      // In saved view, removing means the card itself should disappear.
      if (currentMode === 'saved' && !findSaved(programText)) {
        card.remove();
        if (!gallery.querySelector('.card')) renderSavedEmptyHint();
      }
    });
    saveBtn._refreshSaved = skin;
    actionRow.appendChild(saveBtn);
  }

  if (zcodeSupported) {
    const shareBtn = makeBtn('📋');
    shareBtn.title = 'Copy a permalink to this program to the clipboard';
    shareBtn.addEventListener('click', async () => {
      shareBtn.disabled = true;
      try {
        const url = new URL(location.href);
        url.searchParams.set('zcode', await encodeZcode(programText ?? programEl.value));
        await navigator.clipboard.writeText(url.toString());
        shareBtn.textContent = '✓';
        setTimeout(() => { shareBtn.textContent = '📋'; }, 1200);
      } catch (e) {
        shareBtn.textContent = '⚠';
        setTimeout(() => { shareBtn.textContent = '📋'; }, 1500);
        console.error('share failed', e);
      } finally {
        shareBtn.disabled = false;
      }
    });
    actionRow.appendChild(shareBtn);
  }

  info.appendChild(dlRow);
  info.appendChild(actionRow);

  // Use-as-baseline + show-program toggle. Hidden on the Original card —
  // the editor already shows that program, so these would duplicate state.
  if (programText && !isOriginal) {
    const useBtn = makeBtn('↑');
    useBtn.title = 'Use as baseline (copy program to the editor)';
    useBtn.addEventListener('click', () => {
      programEl.value = programText;
      programEl.scrollIntoView({ behavior: 'smooth' });
    });
    actionRow.appendChild(useBtn);

    const pre = document.createElement('pre');
    pre.className = 'program-pre';
    pre.textContent = programText;

    const toggleBtn = makeBtn('▶');
    toggleBtn.title = 'Show program text';
    toggleBtn.addEventListener('click', () => {
      const visible = pre.classList.toggle('show');
      toggleBtn.textContent = visible ? '▼' : '▶';
    });
    actionRow.appendChild(toggleBtn);

    info.appendChild(pre);
  }

  card.appendChild(canvas);
  card.appendChild(info);
  container.appendChild(card);
}

function addSectionHeader(container, text) {
  const el = document.createElement('div');
  el.className = 'gallery-section-header';
  el.textContent = text;
  container.appendChild(el);
}

// ── Download helpers ──────────────────────────────────────────────────────────

function downloadPng(canvas, label) {
  canvas.toBlob(blob => {
    const a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = slugify(label) + '.png';
    a.click();
    URL.revokeObjectURL(a.href);
  }, 'image/png');
}

async function downloadJxl(programText, label) {
  const res = await fetch('/api/download/jxl', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ program_text: programText }),
  });
  if (!res.ok) throw new Error(await res.text() || `HTTP ${res.status}`);
  const blob = await res.blob();
  const a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = slugify(label) + '.jxl';
  a.click();
  URL.revokeObjectURL(a.href);
}

function fmtBytes(n) {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(2)} MB`;
}

function slugify(s) {
  return s.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'artxl';
}

function makeBtn(text) {
  const b = document.createElement('button');
  b.textContent = text;
  b.className = 'dl-btn';
  return b;
}

// ── Share-link (zcode) ────────────────────────────────────────────────────────

// Format: base64url(deflateRaw(program_text)). Compatible with the ?zcode=
// permalinks used by jpegxl.info, jxl-art.surma.technology, etc. — so links
// made here work there and vice versa. Raw DEFLATE matches Python's
// zlib.{de,}compress with wbits=-15.

async function encodeZcode(text) {
  const bytes = new TextEncoder().encode(text);
  const cs = new CompressionStream('deflate-raw');
  const w = cs.writable.getWriter();
  w.write(bytes); w.close();
  const out = new Uint8Array(await new Response(cs.readable).arrayBuffer());
  let bin = '';
  for (const b of out) bin += String.fromCharCode(b);
  return btoa(bin).replaceAll('+', '-').replaceAll('/', '_').replaceAll('=', '');
}

async function decodeZcode(zcode) {
  const padded = zcode.replaceAll('-', '+').replaceAll('_', '/')
    + '='.repeat((4 - zcode.length % 4) % 4);
  const bin = atob(padded);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  const ds = new DecompressionStream('deflate-raw');
  const w = ds.writable.getWriter();
  w.write(bytes); w.close();
  return new TextDecoder().decode(await new Response(ds.readable).arrayBuffer());
}

const zcodeSupported = typeof CompressionStream !== 'undefined'
  && typeof DecompressionStream !== 'undefined';

initSavedIdCounter();
updateSavedBtnLabel();
main();

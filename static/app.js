const gallery     = document.getElementById('gallery');
const programEl   = document.getElementById('program');
const programHl   = document.getElementById('program-hl').firstElementChild;
const renderSingleBtn   = document.getElementById('render-single-btn');
const renderBtn         = document.getElementById('render-btn');
const renderCompoundBtn = document.getElementById('render-compound-btn');
const sizeSegments      = document.getElementById('size-segments');
const aspectSegments    = document.getElementById('aspect-segments');
const randomBtn         = document.getElementById('random-btn');
const randomSimpleBtn   = document.getElementById('random-simple-btn');
const randomComplexBtn  = document.getElementById('random-complex-btn');
const galleryBtn        = document.getElementById('gallery-btn');
const savedBtn          = document.getElementById('saved-btn');
const errorMsg    = document.getElementById('error-msg');

// ── Editor draft (localStorage) ───────────────────────────────────────────────

// The textarea content is persisted on every input (debounced) and restored
// on page load — so a refresh in the middle of an edit doesn't lose work.
// We do NOT auto-render the restored draft; the user clicks Render.
const DRAFT_KEY = 'artxl.editor.draft';
const DRAFT_DEBOUNCE_MS = 500;

function saveDraft(text) {
  try { localStorage.setItem(DRAFT_KEY, text); } catch {}
}
function loadDraft() {
  try { return localStorage.getItem(DRAFT_KEY); } catch { return null; }
}

// ── Program editor syntax highlighting (jxl-art) ──────────────────────────
// The jxl-art format is line-oriented: a header block (`KEY value` lines),
// a blank line separator, optional spline blobs, then the decision-tree body
// (`if VAR > NUM` or `- PRED [+ NUM | - NUM | NUM]`). There are no parens,
// no quoted symbols, and no comments — so the tokenizer keys off line shape.
const JXL_HEADER_KEYS = new Set([
  'Bitdepth', 'Orientation', 'RCT', 'Channels', 'Width', 'Height',
]);
const JXL_LOWER_VARS = new Set(['x', 'y', 'c']);   // .tk-var
// Predictor and uppercase condition vars share `.tk-fn`; the set is intentionally
// open — we treat any UPPER-leading token in the body as predictor-like.

function escHtml(s) {
  return s.replace(/[&<>]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;' }[c]));
}

// Render one body token. `lineLead` is true if this is the first non-ws token
// on its line — only then can `if` / `-` count as keywords.
function tokenBody(tok, lineLead) {
  if (lineLead && (tok === 'if' || tok === '-')) {
    return `<span class="tk-kw">${escHtml(tok)}</span>`;
  }
  if (tok === '>') return `<span class="tk-paren">&gt;</span>`;
  if (/^-?\d+$/.test(tok)) return `<span class="tk-num">${escHtml(tok)}</span>`;
  if (JXL_LOWER_VARS.has(tok)) return `<span class="tk-var">${escHtml(tok)}</span>`;
  // Predictor names / uppercase vars (`W`, `N`, `WGH`, `AvgN+NW`, `Set`, …).
  if (/^[A-Z]/.test(tok)) return `<span class="tk-fn">${escHtml(tok)}</span>`;
  return escHtml(tok);
}

function tokenizeBodyLine(line) {
  // Walk whitespace-delimited tokens, preserving inter-token whitespace.
  const parts = line.split(/(\s+)/);
  let lead = true;
  let out = '';
  for (const p of parts) {
    if (p === '') continue;
    if (/^\s+$/.test(p)) { out += p; continue; }
    out += tokenBody(p, lead);
    lead = false;
  }
  return out;
}

function highlightProgram(src) {
  const lines = src.split('\n');
  // Find the blank line that separates header from body (first empty line).
  let headerEnd = lines.findIndex(l => l.trim() === '');
  if (headerEnd === -1) headerEnd = lines.length;

  let out = '';
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (i < headerEnd) {
      // Header line: `KEY value` (or just `KEY`). Highlight key as keyword,
      // numeric value as a number. Anything else passes through.
      const m = line.match(/^(\s*)([A-Za-z_][A-Za-z0-9_]*)(\s+)(.*)$/);
      if (m) {
        const [, ws, key, sep, rest] = m;
        const keyHtml = JXL_HEADER_KEYS.has(key) || /^[A-Z]/.test(key)
          ? `<span class="tk-kw">${escHtml(key)}</span>`
          : escHtml(key);
        const restHtml = /^-?\d+(?:\.\d+)?$/.test(rest)
          ? `<span class="tk-num">${escHtml(rest)}</span>`
          : escHtml(rest);
        out += `${ws}${keyHtml}${sep}${restHtml}`;
      } else {
        const m2 = line.match(/^(\s*)([A-Za-z_][A-Za-z0-9_]*)\s*$/);
        if (m2) {
          out += `${m2[1]}<span class="tk-kw">${escHtml(m2[2])}</span>`;
        } else {
          out += escHtml(line);
        }
      }
    } else if (i === headerEnd) {
      // Blank separator line.
      out += escHtml(line);
    } else {
      out += tokenizeBodyLine(line);
    }
    if (i < lines.length - 1) out += '\n';
  }
  // A trailing newline needs a placeholder so the <pre> keeps the last line's
  // height in sync with the textarea.
  return out + '\n';
}

function syncHighlight() {
  programHl.innerHTML = highlightProgram(programEl.value);
}
function syncHighlightScroll() {
  programHl.parentElement.scrollTop = programEl.scrollTop;
  programHl.parentElement.scrollLeft = programEl.scrollLeft;
}

// Set the program text AND refresh the highlight AND persist the draft.
// Use this everywhere instead of assigning programEl.value directly — that
// path doesn't fire input events, so without this the highlight and draft
// would drift from what's actually shown.
function setProgram(text) {
  programEl.value = text;
  saveDraft(text);
  syncHighlight();
}

let draftSaveTimer = null;
programEl.addEventListener('input', () => {
  syncHighlight();
  clearTimeout(draftSaveTimer);
  draftSaveTimer = setTimeout(() => saveDraft(programEl.value), DRAFT_DEBOUNCE_MS);
});
programEl.addEventListener('scroll', syncHighlightScroll);

// First-load draft restoration. A `?zcode=…` permalink trumps the saved
// draft — the user is trying to view that specific program. `initialLoadDraftRestored`
// records whether we restored a draft on this load so the first /api/generate
// stream's "original" item doesn't clobber it.
let initialLoadDraftRestored = false;
{
  const zcode = new URLSearchParams(location.search).get('zcode');
  const draft = loadDraft();
  if (draft && !zcode) {
    programEl.value = draft;
    initialLoadDraftRestored = true;
  }
  syncHighlight();
}

const compareBar  = document.getElementById('compare-bar');
const compareImgs = document.getElementById('compare-images');
const compareClear = document.getElementById('compare-clear');
const compareToggle = document.getElementById('compare-toggle');
const compareCount = document.getElementById('compare-count');
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
  // thumbnails have been downsampled server-side and benefit from an
  // on-demand native-resolution render.
  if (!programText) return;

  const ctrl = new AbortController();
  zoomAbort = ctrl;
  setZoomStatus('loading full resolution…');

  fetchSingleRender(programText, ctrl.signal)
    .then(payload => {
      if (zoomAbort !== ctrl || !zoomModal.classList.contains('open')) return;
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

// ── Keyboard shortcuts + help overlay ─────────────────────────────────────────
const helpBtn        = document.getElementById('help-btn');
const shortcutsModal = document.getElementById('shortcuts-modal');
const shortcutsClose = document.getElementById('shortcuts-close');

function openShortcuts()  { shortcutsModal.classList.add('open'); }
function closeShortcuts() { shortcutsModal.classList.remove('open'); }

helpBtn.addEventListener('click', openShortcuts);
shortcutsClose.addEventListener('click', closeShortcuts);
shortcutsModal.addEventListener('click', e => {
  if (e.target === shortcutsModal) closeShortcuts();
});

// Is the user currently typing into a field? Single-key shortcuts back off
// when they are, so typing a program never triggers navigation.
function isTyping(el) {
  return el && (el.tagName === 'TEXTAREA' || el.tagName === 'INPUT' || el.isContentEditable);
}

document.addEventListener('keydown', e => {
  if (e.key === 'Escape') { closeZoom(); closeShortcuts(); return; }

  if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
    e.preventDefault();
    renderBtn.click();
    return;
  }

  if (isTyping(e.target) || e.metaKey || e.ctrlKey || e.altKey || e.isComposing) return;

  switch (e.key.toLowerCase()) {
    case '?': openShortcuts(); break;
    case 'r': e.preventDefault(); randomBtn.click(); break;
    case 'g': e.preventDefault(); galleryBtn.click(); break;
    case 's': e.preventDefault(); savedBtn.click(); break;
    case 'e': e.preventDefault(); programEl.focus(); break;
  }
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

function initSavedIdCounter() {
  const arr = loadSaved();
  savedIdCounter = arr.reduce((m, e) => Math.max(m, e.id || 0), 0);
}

// Drop saves from the previous schema (which embedded full webp_b64 payloads
// and blew the 5MB localStorage quota). Best-effort; ignored on failure.
try { localStorage.removeItem('artxl.saved.v1'); } catch {}

// ── Comparison state ──────────────────────────────────────────────────────────

const pinned = new Map();
let pinId = 0;

function togglePin(srcCanvas, label) {
  for (const [id, { srcCanvas: c }] of pinned) {
    if (c === srcCanvas) { unpin(id); return; }
  }
  const id = pinId++;

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
  refreshAllCompareButtons();
  compareBar.style.display = 'block';
  updateCompareCount();
  requestAnimationFrame(syncBarPadding);
}

function unpin(id) {
  const { srcCanvas, el } = pinned.get(id);
  srcCanvas.classList.remove('pinned');
  el.remove();
  pinned.delete(id);
  refreshAllCompareButtons();
  updateCompareCount();
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

// ── Collapsible comparison bar ────────────────────────────────────────────
// Default state is collapsed: a small floating badge bottom-left showing the
// pinned count. Clicking it expands the full image strip; the chevron (or
// another click) collapses it back. The choice persists across sessions.
const COMPARE_COLLAPSED_KEY = 'artxl:compareCollapsed';
let compareCollapsed = localStorage.getItem(COMPARE_COLLAPSED_KEY) !== 'false';

function applyCompareCollapsed() {
  compareBar.classList.toggle('collapsed', compareCollapsed);
  if (compareBar.style.display !== 'none') requestAnimationFrame(syncBarPadding);
}

function setCompareCollapsed(collapsed) {
  compareCollapsed = collapsed;
  localStorage.setItem(COMPARE_COLLAPSED_KEY, String(collapsed));
  applyCompareCollapsed();
}

function updateCompareCount() {
  compareCount.textContent = pinned.size;
}

compareToggle.addEventListener('click', () => setCompareCollapsed(!compareCollapsed));
applyCompareCollapsed();

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
  // Only the expanded drawer reserves layout space; the collapsed badge floats.
  setBarPadding(compareCollapsed ? '' : compareBar.offsetHeight + 'px');
}

// ── Streaming fetch ───────────────────────────────────────────────────────────

// Build a placeholder card that pulses while a stream item is in flight.
// The skeleton mirrors a real card's structure (canvas + head + action row)
// so when the rendered card replaces it via fillSlot, no surrounding card
// shifts. Pass compact=true for gallery cards, whose real head holds only
// the small byte-size chip (no label).
function makeSkeleton(compact, ratio) {
  const ph = document.createElement('div');
  ph.className = 'card skeleton';
  const c = document.createElement('div');
  c.className = 'skeleton-canvas';
  // Match the upcoming image's shape so the real card replaces the ghost with
  // no reflow (defaults to the CSS 1/1 when no ratio is known).
  if (ratio) c.style.aspectRatio = ratio;
  ph.appendChild(c);
  const info = document.createElement('div');
  info.className = 'info';
  const head = document.createElement('div');
  head.className = 'card-head';
  const line = document.createElement('span');
  line.className = 'skeleton-line' + (compact ? ' compact' : '');
  head.appendChild(line);
  info.appendChild(head);
  const row = document.createElement('div');
  row.className = 'skeleton-row';
  for (const variant of ['compare', 'icon', 'icon', 'overflow']) {
    const b = document.createElement('span');
    b.className = `skeleton-btn skeleton-btn-${variant}`;
    row.appendChild(b);
  }
  info.appendChild(row);
  ph.appendChild(info);
  return ph;
}

function fillSlot(slot, label, image, isOriginal, programText, warning, hideLabel, labelTitle) {
  if (!slot || !slot.parentNode) {
    renderCard(gallery, label, image, isOriginal, programText, warning, hideLabel, labelTitle);
    return;
  }
  const tmp = document.createElement('div');
  renderCard(tmp, label, image, isOriginal, programText, warning, hideLabel, labelTitle);
  slot.replaceWith(tmp.firstElementChild);
}

async function streamFrom(url, method, body, size = 0, signal, prefill) {
  let mutationCount = 0;
  let rendered = 0;
  let simpleSectionAdded = false;
  let compoundSectionAdded = false;
  let compoundIdx = 0;

  // Placeholder bookkeeping. Each render mode reserves its slots the moment
  // we know how many to expect:
  //   • mutations  — when the `original` arrives with mutation_count
  //   • batch      — pre-reserved up front (prefill) so ghosts show instantly,
  //                  then reconciled against .total
  //   • gallery    — on the first gallery_image (uses .total)
  // The slot is then replaced in-place when its payload lands.
  const mutationQueue = [];
  const batchSlots = new Map();
  const gallerySlots = new Map();
  let batchReserved = false;

  // Reserve the batch ghost cards before the request resolves so they appear
  // the instant the action starts, already at the right aspect (no wait for
  // the first render, no reflow). The gallery was wiped by runAction.
  const batchRatio = prefill && prefill.ratio;
  if (prefill) {
    for (let i = 0; i < prefill.count; i++) {
      const ph = makeSkeleton(false, batchRatio);
      batchSlots.set(i, ph);
      gallery.appendChild(ph);
    }
  }

  const fullUrl = size ? url + (url.includes('?') ? '&' : '?') + `size=${size}` : url;
  const opts = { method, signal };
  if (body) {
    opts.headers = { 'Content-Type': 'application/json' };
    opts.body = JSON.stringify(size ? { ...body, size } : body);
  }
  const res = await fetch(fullUrl, opts);
  if (!res.ok) {
    const msg = await res.text();
    throw new Error(msg || `HTTP ${res.status}`);
  }

  function insertSectionHeader(text) {
    const hdr = document.createElement('div');
    hdr.className = 'gallery-section-header';
    hdr.textContent = text;
    const anchor = mutationQueue[0];
    if (anchor && anchor.parentNode === gallery) {
      gallery.insertBefore(hdr, anchor);
    } else {
      gallery.appendChild(hdr);
    }
  }

  for await (const item of readNdjson(res)) {
    if (item.type === 'batch_image') {
      // Top up any slots not already pre-filled — exactly once (guarded by
      // batchReserved, since slots get deleted as they fill). No-op when
      // prefill already covered the count; fallback if it didn't.
      if (!batchReserved) {
        batchReserved = true;
        for (let i = batchSlots.size; i < item.total; i++) {
          const ph = makeSkeleton(false, batchRatio);
          batchSlots.set(i, ph);
          gallery.appendChild(ph);
        }
      }
      fillSlot(batchSlots.get(item.index),
        `Random ${item.index + 1}`, item.image, false, item.program_text);
      batchSlots.delete(item.index);
      rendered++;
      reportProgress(`${rendered} / ${item.total}`);
    } else if (item.type === 'original') {
      mutationCount = item.mutation_count;
      // First-load restoration: the draft is already in the textarea; the
      // initial /api/generate stream's first 'original' shouldn't clobber it.
      if (initialLoadDraftRestored) {
        initialLoadDraftRestored = false;
      } else {
        setProgram(item.program_text);
      }
      renderCard(gallery, 'Original', item.image, true, item.program_text);
      rendered++;
      // For homogeneous render modes (all-simple "mutations" or all-compound
      // "compound20") we know the section up front — insert its header before
      // reserving the skeletons so it doesn't pop in once the first mutation
      // lands. Mixed streams fall back to lazy insertion below.
      const mode = body && body.mode;
      if (mutationCount > 0 && mode === 'mutations') {
        addSectionHeader(gallery, 'Simple mutations');
        simpleSectionAdded = true;
      } else if (mutationCount > 0 && mode === 'compound20') {
        addSectionHeader(gallery, 'Compound mutations');
        compoundSectionAdded = true;
      }
      const mutRatio = item.image && item.image.width && item.image.height
        ? `${item.image.width} / ${item.image.height}`
        : undefined;
      for (let i = 0; i < mutationCount; i++) {
        const ph = makeSkeleton(false, mutRatio);
        mutationQueue.push(ph);
        gallery.appendChild(ph);
      }
      reportProgress(`${rendered} / ${mutationCount + 1}`);
    } else if (item.type === 'mutation') {
      if (!item.compound && !simpleSectionAdded) {
        insertSectionHeader('Simple mutations');
        simpleSectionAdded = true;
      }
      if (item.compound && !compoundSectionAdded) {
        insertSectionHeader('Compound mutations');
        compoundSectionAdded = true;
      }
      // Compound mutation labels are kebab-cased mutation chains like
      // "mutate-leaf + swap-children + invert-comparison" — readable but
      // wraps to multiple lines. Rename to "Compound N" and keep the full
      // chain in the title attr (and persist it on save).
      let displayLabel = item.label;
      let labelTitle = null;
      if (item.compound) {
        compoundIdx++;
        displayLabel = `Compound ${compoundIdx}`;
        labelTitle = item.label;
      }
      fillSlot(mutationQueue.shift(),
        displayLabel, item.image, false, item.program_text, item.warning, false, labelTitle);
      rendered++;
      reportProgress(`${rendered} / ${mutationCount + 1}`);
    } else if (item.type === 'gallery_image') {
      if (gallerySlots.size === 0) {
        for (let i = 0; i < item.total; i++) {
          const ph = makeSkeleton(true);
          gallerySlots.set(i, ph);
          gallery.appendChild(ph);
        }
      }
      fillSlot(gallerySlots.get(item.index),
        item.name, item.image, false, item.program_text, null, true);
      gallerySlots.delete(item.index);
      rendered++;
      reportProgress(`${rendered} / ${item.total}`);
    } else if (item.type === 'done') {
      for (const ph of mutationQueue) ph.remove();
      for (const ph of batchSlots.values()) ph.remove();
      for (const ph of gallerySlots.values()) ph.remove();
    }
  }
}

// ── Data loading ──────────────────────────────────────────────────────────────

function main() {
  runAction(null, null, signal => mainFn(signal));
}

async function mainFn(signal) {
  const zcode = new URLSearchParams(location.search).get('zcode');
  if (zcode && zcodeSupported) {
    try {
      const programText = await decodeZcode(zcode);
      await streamFrom('/api/render', 'POST',
        { program_text: programText, mode: 'single' }, 0, signal);
      return;
    } catch (e) {
      if (e.name === 'AbortError') throw e;
      console.error('bad zcode, falling back to /api/generate', e);
      errorMsg.textContent = 'Share link is invalid — showing a random program instead.';
    }
  }
  await streamFrom('/api/generate', 'GET', null, 0, signal);
}

let currentMode = 'normal'; // 'normal' | 'gallery' | 'saved'

function resetModeToggles() {
  gallery.classList.remove('gallery-mode');
  if (savedAbort) { savedAbort.abort(); savedAbort = null; }
  currentMode = 'normal';
  galleryBtn.textContent = 'Gallery';
  updateSavedBtnLabel();
}

// ── runAction: supersedable in-flight tracker ─────────────────────────────
// While a top-level action is in flight, every other action button stays
// clickable. Clicking a different one aborts the current stream and starts
// the new one; clicking the busy button (or the inline cancel chip) aborts
// without starting anything new. Progress counts are written onto the busy
// button via `reportProgress` so the status line can stay empty.

let inflight = null; // { controller, btn, originalText, prevMinWidth }
let activeBusyBtn = null;
// Cards detached when opening Gallery/Saved from normal mode. Re-attached
// when the panel closes so a half-finished randomize/render isn't lost.
let stashedCards = null;

// Progress counts ride on the pressed button (see runAction's width-freeze).
// When there's no pressed button — initial page load via main() — there's
// nowhere to render progress, and we deliberately stay silent rather than
// reserving layout for a status line.
function reportProgress(text) {
  if (activeBusyBtn) activeBusyBtn.textContent = text;
}

function restoreBtn(rec) {
  if (!rec.btn) return;
  rec.btn.textContent = rec.originalText;
  rec.btn.style.minWidth = rec.prevMinWidth;
}

function runAction(btn, label, fn) {
  if (inflight) {
    const prev = inflight;
    inflight = null;
    prev.controller.abort();
    restoreBtn(prev);
    activeBusyBtn = null;
    if (prev.btn === btn) return; // self-cancel only
  }

  const orig = btn ? btn.textContent : null;
  const prevMinWidth = btn ? btn.style.minWidth : '';
  if (btn && label != null) {
    // Freeze the button at (at least) its resting width so the changing
    // progress count inside ("3 / 20" → "12 / 20") doesn't make it twitch.
    const restWidth = btn.offsetWidth;
    btn.textContent = label;
    btn.style.minWidth = Math.max(restWidth, btn.offsetWidth) + 'px';
  }
  activeBusyBtn = btn || null;
  errorMsg.textContent = '';
  // Opening Gallery/Saved from a populated normal view detaches (but keeps
  // alive) the current cards so closing the panel restores them — no need to
  // re-run /api/generate. Pins reference the same canvases, so they survive
  // the round-trip too.
  const enteringPanel =
    (btn === galleryBtn || btn === savedBtn) && currentMode === 'normal';
  if (enteringPanel && stashedCards === null && gallery.children.length > 0) {
    // Drop unfilled skeletons — restoring them would just show ghost cards.
    stashedCards = Array.from(gallery.children)
      .filter(el => !el.classList.contains('skeleton'));
    gallery.replaceChildren();
    if (stashedCards.length === 0) stashedCards = null;
  } else {
    gallery.innerHTML = '';
    clearAllPins();
    stashedCards = null;
  }
  if (btn !== galleryBtn && btn !== savedBtn) resetModeToggles();

  const controller = new AbortController();
  const my = { controller, btn, originalText: orig, prevMinWidth };
  inflight = my;

  fn(controller.signal)
    .catch(e => {
      if (e.name === 'AbortError') return;
      errorMsg.textContent = `Error: ${e.message}`;
    })
    .finally(() => {
      if (inflight !== my) return; // superseded — the new owner manages UI
      restoreBtn(my);
      activeBusyBtn = null;
      inflight = null;
    });
}

// ── Render-button bindings ────────────────────────────────────────────────

// Shared render size, controlled by the segmented picker. 0 = native (1024),
// 320 = preview, 2048 = large. Applies to all three Render buttons.
let renderSize = 0;
sizeSegments.querySelectorAll('.size-seg').forEach(seg => {
  seg.addEventListener('click', () => {
    renderSize = parseInt(seg.dataset.size, 10) || 0;
    sizeSegments.querySelectorAll('.size-seg').forEach(s => {
      const active = s === seg;
      s.classList.toggle('active', active);
      s.setAttribute('aria-checked', active ? 'true' : 'false');
    });
  });
});

// Aspect ratio for *generated* canvases (Randomize only). Combines with the
// size segment: size = the canvas's longest edge, aspect = the W:H shape.
let genAspect = 'square';
aspectSegments.querySelectorAll('.size-seg').forEach(seg => {
  seg.addEventListener('click', () => {
    genAspect = seg.dataset.aspect || 'square';
    aspectSegments.querySelectorAll('.size-seg').forEach(s => {
      const active = s === seg;
      s.classList.toggle('active', active);
      s.setAttribute('aria-checked', active ? 'true' : 'false');
    });
  });
});

// Generated-canvas dimensions from the size×aspect selectors, or null for the
// default (1:1 + native/1024) — where the server picks 1024² + occasional
// pixel-mode. Explicit shape/size uses a 16:9 ratio.
function genDims() {
  const base = renderSize || 1024;
  if (genAspect === 'square' && base === 1024) return null;
  const short = Math.round(base * 9 / 16);
  if (genAspect === 'wide') return { w: base, h: short };
  if (genAspect === 'tall') return { w: short, h: base };
  return { w: base, h: base };
}

// CSS aspect-ratio string for the upcoming generated cards (square by default).
function genAspectRatio() {
  const d = genDims();
  return d ? `${d.w} / ${d.h}` : '1 / 1';
}

// Build the random-batch URL. No dims for the default square+1024 case so the
// server keeps its pixel-mode; otherwise the fixed canvas from genDims().
function randomUrl(complexity) {
  const url = `/api/random/batch?complexity=${complexity}`;
  const d = genDims();
  return d ? url + `&width=${d.w}&height=${d.h}` : url;
}

// Must match the server's random_batch COUNT.
const BATCH_COUNT = 20;

function bindSizes(btn, busy, url, method, body) {
  btn.addEventListener('click', () =>
    runAction(btn, busy, signal => streamFrom(url, method, body(), renderSize, signal)));
}

// Random-program generators take a tree-complexity knob. The main Randomize
// button uses default; the Simple/Complex side buttons override the tree
// depth. Whichever you click, progress is shown on the main Randomize button
// (seeded "0 / 20" so it doesn't resize) — the narrow side buttons stay put.
function bindRandom(triggerBtn, complexity) {
  triggerBtn.addEventListener('click', () => runAction(randomBtn, '0 / 20',
    signal => streamFrom(randomUrl(complexity), 'GET', null, 0, signal,
      { count: BATCH_COUNT, ratio: genAspectRatio() })));
}
bindRandom(randomBtn,        1);
bindRandom(randomSimpleBtn,  0);
bindRandom(randomComplexBtn, 2);

bindSizes(renderSingleBtn,
  '…', '/api/render', 'POST', () => ({ program_text: programEl.value, mode: 'single' }));

bindSizes(renderBtn,
  '…', '/api/render', 'POST', () => ({ program_text: programEl.value, mode: 'mutations' }));

bindSizes(renderCompoundBtn,
  '…', '/api/render', 'POST', () => ({ program_text: programEl.value, mode: 'compound20' }));

function openGallery() {
  runAction(galleryBtn, 'Loading…', async signal => {
    gallery.classList.add('gallery-mode');
    currentMode = 'gallery';
    galleryBtn.textContent = 'Close gallery';
    addGalleryCredit(gallery);
    await streamFrom('/api/gallery', 'GET', null, 0, signal);
  });
}

// In-flight saved-view renders, cancelled when the view is closed or the
// user jumps into another mode mid-load.
let savedAbort = null;

function openSaved() {
  runAction(savedBtn, 'Loading…', async signal => {
    gallery.classList.add('gallery-mode');
    currentMode = 'saved';
    savedBtn.textContent = 'Close saved';
    await renderSavedView(signal);
  });
}

async function renderSavedView(outerSignal) {
  const arr = loadSaved().slice().sort((a, b) => b.savedAt - a.savedAt);
  if (arr.length === 0) {
    renderSavedEmptyHint();
    return;
  }

  if (savedAbort) savedAbort.abort();
  const ctrl = new AbortController();
  savedAbort = ctrl;
  // Forward outer (runAction) aborts to the per-card controller so a
  // supersede from another action cleanly cancels the in-flight renders.
  if (outerSignal) {
    if (outerSignal.aborted) ctrl.abort();
    else outerSignal.addEventListener('abort', () => ctrl.abort(), { once: true });
  }

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
        // Progress is reflected by individual card fills; no aggregate line.
        if (activeBusyBtn) activeBusyBtn.textContent = `${done} / ${arr.length}`;
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

function restoreStashed() {
  if (inflight) {
    const prev = inflight;
    inflight = null;
    prev.controller.abort();
    restoreBtn(prev);
    activeBusyBtn = null;
  }
  resetModeToggles();
  gallery.replaceChildren(...stashedCards);
  stashedCards = null;
}

galleryBtn.addEventListener('click', () => {
  if (currentMode === 'gallery') {
    if (stashedCards) restoreStashed();
    else { resetModeToggles(); main(); }
  } else {
    openGallery();
  }
});

savedBtn.addEventListener('click', () => {
  if (currentMode === 'saved') {
    if (stashedCards) restoreStashed();
    else { resetModeToggles(); main(); }
  } else {
    openSaved();
  }
});

// ── Card rendering ────────────────────────────────────────────────────────────

function renderCard(container, label, payload, isOriginal, programText, warning, hideLabel, labelTitle) {
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

  const jxlSizeValue = payload.jxl_size ?? 0;

  // ── Head: label (+badge) on the left, byte size on the right ────────
  const head = document.createElement('div');
  head.className = 'card-head';

  if (!hideLabel) {
    const lbl = document.createElement('span');
    lbl.className = 'label';
    lbl.textContent = label;
    if (labelTitle) {
      lbl.title = labelTitle;
      lbl.classList.add('has-detail');
    }
    if (isOriginal) {
      const badge = document.createElement('span');
      badge.className = 'original-badge';
      badge.textContent = 'original';
      lbl.appendChild(badge);
    }
    head.appendChild(lbl);
  }

  if (jxlSizeValue > 0) {
    const sizeEl = document.createElement('span');
    sizeEl.className = 'jxl-size';
    sizeEl.textContent = fmtBytes(jxlSizeValue);
    sizeEl.title = 'Encoded JXL size';
    head.appendChild(sizeEl);
  }

  if (head.childNodes.length > 0) info.appendChild(head);

  // ── Action row: primary actions visible, rest behind ⋯ overflow ─────
  const actionRow = document.createElement('div');
  actionRow.className = 'action-row';

  const cmpBtn = makeLabeledBtn('⊞', 'Compare');
  cmpBtn.classList.add('cmp-action');
  cmpBtn.title = 'Pin to comparison bar';
  cmpBtn.addEventListener('click', () => togglePin(canvas, label));
  cmpBtn._refreshCompare = () => {
    const isPinned = canvas.classList.contains('pinned');
    cmpBtn.classList.toggle('active', isPinned);
    cmpBtn.querySelector('.text').textContent = isPinned ? 'Pinned' : 'Compare';
  };
  cmpBtn._refreshCompare();
  actionRow.appendChild(cmpBtn);

  if (programText && !isOriginal) {
    const useBtn = makeLabeledBtn('↑', 'Baseline', true);
    useBtn.title = 'Use as baseline (copy program to the editor)';
    useBtn.addEventListener('click', () => {
      setProgram(programText);
      programEl.scrollIntoView({ behavior: 'smooth' });
    });
    actionRow.appendChild(useBtn);
  }

  if (programText) {
    const saveBtn = makeLabeledBtn('☆', 'Save', true);
    const skin = () => {
      const saved = !!findSaved(programText);
      saveBtn.querySelector('.icon').textContent = saved ? '★' : '☆';
      saveBtn.querySelector('.text').textContent = saved ? 'Saved' : 'Save';
      saveBtn.classList.toggle('saved', saved);
      saveBtn.title = saved ? 'Remove from saved' : 'Save this image';
    };
    skin();
    saveBtn.addEventListener('click', () => {
      const existing = findSaved(programText);
      if (existing) {
        removeSaved(existing.id);
      } else if (!addSaved({
        // Persist the detailed label (full mutation chain) rather than the
        // session-local "Compound 5" display name, which has no stable meaning.
        label: labelTitle || label,
        programText,
        jxl_size: payload.jxl_size,
      })) {
        return;
      }
      refreshAllSaveButtons();
      updateSavedBtnLabel();
      if (currentMode === 'saved' && !findSaved(programText)) {
        card.remove();
        if (!gallery.querySelector('.card')) renderSavedEmptyHint();
      }
    });
    saveBtn._refreshSaved = skin;
    actionRow.appendChild(saveBtn);
  }

  // Inline <pre> for "View source program" toggle — placed inside info,
  // toggled via the overflow menu item below.
  let pre = null;
  if (programText && !isOriginal) {
    pre = document.createElement('pre');
    pre.className = 'program-pre';
    pre.textContent = programText;
  }

  // Overflow ⋯ menu — secondary actions live here.
  const overflowWrap = document.createElement('div');
  overflowWrap.className = 'overflow-wrap';
  const overflowBtn = document.createElement('button');
  overflowBtn.className = 'dl-btn overflow-btn';
  overflowBtn.title = 'More actions';
  overflowBtn.textContent = '⋯';
  overflowWrap.appendChild(overflowBtn);

  const menu = document.createElement('div');
  menu.className = 'card-menu';
  menu.addEventListener('click', (e) => e.stopPropagation());

  function addMenuItem(text, fn) {
    const item = document.createElement('button');
    item.className = 'card-menu-item';
    item.textContent = text;
    item.addEventListener('click', (e) => {
      e.stopPropagation();
      fn(item);
    });
    menu.appendChild(item);
    return item;
  }

  addMenuItem('↓ Download PNG', () => {
    menu.classList.remove('open');
    downloadPng(canvas, label);
  });

  if (jxlSizeValue > 0) {
    addMenuItem(`↓ Download JXL · ${fmtBytes(jxlSizeValue)}`, async (item) => {
      const orig = item.textContent;
      item.textContent = '… preparing JXL';
      try {
        await downloadJxl(programText ?? programEl.value, label);
        menu.classList.remove('open');
        item.textContent = orig;
      } catch (e) {
        item.textContent = '⚠ ' + e.message;
        setTimeout(() => { item.textContent = orig; menu.classList.remove('open'); }, 1500);
      }
    });
  }

  if (zcodeSupported && programText) {
    addMenuItem('🔗 Copy share link', async (item) => {
      const orig = item.textContent;
      try {
        const url = new URL(location.href);
        url.searchParams.set('zcode', await encodeZcode(programText));
        await navigator.clipboard.writeText(url.toString());
        item.textContent = '✓ Link copied';
      } catch (e) {
        console.error('share failed', e);
        item.textContent = '⚠ Copy failed';
      }
      setTimeout(() => {
        item.textContent = orig;
        menu.classList.remove('open');
      }, 900);
    });
  }

  if (pre) {
    const viewItem = addMenuItem('▶ View source program', () => {
      const visible = pre.classList.toggle('show');
      viewItem.textContent = visible ? '▼ Hide source program' : '▶ View source program';
      menu.classList.remove('open');
    });
  }

  overflowWrap.appendChild(menu);
  overflowBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    document.querySelectorAll('.card-menu.open').forEach(m => {
      if (m !== menu) m.classList.remove('open');
    });
    menu.classList.toggle('open');
  });

  actionRow.appendChild(overflowWrap);
  info.appendChild(actionRow);
  if (pre) info.appendChild(pre);

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

// Labeled button (icon + text) used on cards. Refresh helpers read the
// .icon/.text spans by class so saved/pinned state can be toggled without
// rebuilding the element. Pass collapsible=true to let the text hide on
// narrow cards (container query) and reappear when the card is wide.
function makeLabeledBtn(icon, text, collapsible) {
  const b = document.createElement('button');
  b.className = 'dl-btn labeled' + (collapsible ? ' collapsible' : '');
  const ic = document.createElement('span');
  ic.className = 'icon';
  ic.textContent = icon;
  const tx = document.createElement('span');
  tx.className = 'text';
  tx.textContent = text;
  b.appendChild(ic);
  b.appendChild(tx);
  return b;
}

// Walk every Compare button on the page and let it re-derive its visual
// state from `canvas.classList.contains('pinned')`. Called from
// togglePin / unpin / clearAllPins so the compare bar's ✕ button and the
// card's own Compare button never get out of sync.
function refreshAllCompareButtons() {
  for (const b of gallery.querySelectorAll('.dl-btn.labeled')) {
    if (typeof b._refreshCompare === 'function') b._refreshCompare();
  }
}

// Close any open card overflow menu on outside click. Menu clicks are
// stopped at the menu element so they don't bubble up to this handler.
document.addEventListener('click', () => {
  document.querySelectorAll('.card-menu.open').forEach(m => m.classList.remove('open'));
});

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

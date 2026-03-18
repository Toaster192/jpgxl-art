const statusEl    = document.getElementById('status');
const gallery     = document.getElementById('gallery');
const programEl   = document.getElementById('program');
const renderSingleBtn        = document.getElementById('render-single-btn');
const renderSinglePreviewBtn = document.getElementById('render-single-preview-btn');
const renderBtn              = document.getElementById('render-btn');
const renderPreviewBtn       = document.getElementById('render-preview-btn');
const renderCompoundBtn      = document.getElementById('render-compound-btn');
const renderCompoundPreviewBtn = document.getElementById('render-compound-preview-btn');
const randomBtn          = document.getElementById('random-btn');
const randomPreviewBtn   = document.getElementById('random-preview-btn');
const random20Btn        = document.getElementById('random20-btn');
const random20PreviewBtn = document.getElementById('random20-preview-btn');
const errorMsg    = document.getElementById('error-msg');
const compareBar  = document.getElementById('compare-bar');
const compareImgs = document.getElementById('compare-images');
const compareClear = document.getElementById('compare-clear');
const zoomModal   = document.getElementById('zoom-modal');
const zoomCanvas  = document.getElementById('zoom-canvas');

// ── Zoom modal ─────────────────────────────────────────────────────────────────

function showZoom(srcCanvas) {
  zoomCanvas.width  = srcCanvas.width;
  zoomCanvas.height = srcCanvas.height;
  zoomCanvas.getContext('2d').drawImage(srcCanvas, 0, 0);
  zoomModal.classList.add('open');
}

zoomModal.addEventListener('click', () => zoomModal.classList.remove('open'));

document.addEventListener('keydown', e => {
  if (e.key === 'Escape') zoomModal.classList.remove('open');
});

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
  c.title = 'Click to zoom';
  c.addEventListener('click', () => showZoom(c));

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
    document.body.style.paddingBottom = '';
  } else {
    requestAnimationFrame(syncBarPadding);
  }
}

function clearAllPins() {
  for (const [id] of [...pinned]) unpin(id);
}

compareClear.addEventListener('click', clearAllPins);

function syncBarPadding() {
  document.body.style.paddingBottom = compareBar.offsetHeight + 'px';
}

// ── Streaming fetch ───────────────────────────────────────────────────────────

async function streamFrom(url, method, body, preview = false) {
  const fullUrl = preview ? url + (url.includes('?') ? '&' : '?') + 'preview=true' : url;
  const opts = { method };
  if (body) {
    opts.headers = { 'Content-Type': 'application/json' };
    opts.body = JSON.stringify(preview ? { ...body, preview: true } : body);
  }
  const res = await fetch(fullUrl, opts);
  if (!res.ok) {
    const msg = await res.text();
    throw new Error(msg || `HTTP ${res.status}`);
  }

  const reader = res.body.getReader();
  const decoder = new TextDecoder();
  let buf = '';
  let mutationCount = 0;
  let rendered = 0;
  let simpleSectionAdded = false;
  let compoundSectionAdded = false;

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    buf += decoder.decode(value, { stream: true });

    let nl;
    while ((nl = buf.indexOf('\n')) !== -1) {
      const line = buf.slice(0, nl).trim();
      buf = buf.slice(nl + 1);
      if (!line) continue;
      let item;
      try { item = JSON.parse(line); } catch { continue; }

      if (item.type === 'batch_image') {
        renderCard(gallery, `Random ${item.index + 1}`, item.image, false, item.program_text);
        rendered++;
        statusEl.textContent = `Generated ${rendered} / ${item.total}…`;
        if (rendered === item.total) statusEl.textContent = `Generated ${rendered} random images.`;
      } else if (item.type === 'original') {
        mutationCount = item.mutation_count;
        programEl.value = item.program_text;
        renderCard(gallery, 'Original', item.image, true, null);
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
      } else if (item.type === 'done') {
        statusEl.textContent = `Rendered ${rendered} images.`;
      }
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
    await streamFrom('/api/generate', 'GET', null, false);
  } catch (e) {
    statusEl.textContent = `Error: ${e.message}`;
  }
}

const allBtns = [
  renderSingleBtn, renderSinglePreviewBtn,
  renderBtn, renderPreviewBtn,
  renderCompoundBtn, renderCompoundPreviewBtn,
  randomBtn, randomPreviewBtn,
  random20Btn, random20PreviewBtn,
];

function withBusy(btn, label, fn) {
  const orig = btn.textContent;
  btn.textContent = label;
  allBtns.forEach(b => b.disabled = true);
  errorMsg.textContent = '';
  gallery.innerHTML = '';
  clearAllPins();
  fn().catch(e => { errorMsg.textContent = `Error: ${e.message}`; })
      .finally(() => { allBtns.forEach(b => b.disabled = false); btn.textContent = orig; });
}

random20Btn.addEventListener('click', () =>
  withBusy(random20Btn, 'Generating…', () =>
    streamFrom('/api/random/batch', 'GET', null, false)));

random20PreviewBtn.addEventListener('click', () =>
  withBusy(random20PreviewBtn, '…', () =>
    streamFrom('/api/random/batch', 'GET', null, true)));

randomBtn.addEventListener('click', () =>
  withBusy(randomBtn, 'Randomizing…', () =>
    streamFrom('/api/random', 'GET', null, false)));

randomPreviewBtn.addEventListener('click', () =>
  withBusy(randomPreviewBtn, '…', () =>
    streamFrom('/api/random', 'GET', null, true)));

renderSingleBtn.addEventListener('click', () =>
  withBusy(renderSingleBtn, 'Rendering…', () =>
    streamFrom('/api/render', 'POST', { program_text: programEl.value, mode: 'single' }, false)));

renderSinglePreviewBtn.addEventListener('click', () =>
  withBusy(renderSinglePreviewBtn, '…', () =>
    streamFrom('/api/render', 'POST', { program_text: programEl.value, mode: 'single' }, true)));

renderBtn.addEventListener('click', () =>
  withBusy(renderBtn, 'Rendering…', () =>
    streamFrom('/api/render', 'POST', { program_text: programEl.value, mode: 'mutations' }, false)));

renderPreviewBtn.addEventListener('click', () =>
  withBusy(renderPreviewBtn, '…', () =>
    streamFrom('/api/render', 'POST', { program_text: programEl.value, mode: 'mutations' }, true)));

renderCompoundBtn.addEventListener('click', () =>
  withBusy(renderCompoundBtn, 'Rendering…', () =>
    streamFrom('/api/render', 'POST', { program_text: programEl.value, mode: 'compound20' }, false)));

renderCompoundPreviewBtn.addEventListener('click', () =>
  withBusy(renderCompoundPreviewBtn, '…', () =>
    streamFrom('/api/render', 'POST', { program_text: programEl.value, mode: 'compound20' }, true)));

// ── Card rendering ────────────────────────────────────────────────────────────

function renderCard(container, label, payload, isOriginal, programText, warning) {
  const card = document.createElement('div');
  card.className = 'card';

  const canvas = document.createElement('canvas');
  canvas.width  = payload.width;
  canvas.height = payload.height;
  const rgba = base64ToUint8Array(payload.rgba_b64);
  const ctx = canvas.getContext('2d');
  ctx.putImageData(new ImageData(new Uint8ClampedArray(rgba), payload.width, payload.height), 0, 0);

  // Click canvas to zoom; compare button pins to comparison bar
  canvas.title = 'Click to zoom';
  canvas.addEventListener('click', () => showZoom(canvas));

  const info = document.createElement('div');
  info.className = 'info';

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

  // Download + compare buttons
  const dlRow = document.createElement('div');
  dlRow.style.cssText = 'display:flex;gap:6px;margin-top:6px;flex-wrap:wrap;align-items:center;';

  const pngBtn = makeBtn('↓ PNG');
  pngBtn.addEventListener('click', () => downloadPng(canvas, label));
  dlRow.appendChild(pngBtn);

  if (payload.jxl_size > 0) {
    const jxlBtn = makeBtn('↓ JXL');
    const jxlSize = document.createElement('span');
    jxlSize.style.cssText = 'font-size:0.7rem;color:#666;align-self:center;';
    jxlSize.textContent = fmtBytes(payload.jxl_size);
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

  const cmpBtn = makeBtn('⊞ compare');
  cmpBtn.title = 'Pin to comparison bar';
  cmpBtn.addEventListener('click', () => togglePin(canvas, label));
  dlRow.appendChild(cmpBtn);

  info.appendChild(dlRow);

  // Program text toggle + use-as-baseline
  if (programText) {
    const actionRow = document.createElement('div');
    actionRow.style.cssText = 'display:flex;gap:8px;align-items:baseline;margin-top:4px;';

    const toggle = document.createElement('a');
    toggle.href = '#';
    toggle.style.cssText = 'font-size:0.7rem;color:#666;';
    toggle.textContent = '▶ show program';

    const pre = document.createElement('pre');
    pre.style.cssText = 'display:none;font-size:0.65rem;color:#9cdcfe;overflow-x:auto;white-space:pre;margin-top:4px;';
    pre.textContent = programText;

    toggle.addEventListener('click', e => {
      e.preventDefault();
      const hidden = pre.style.display === 'none';
      pre.style.display = hidden ? 'block' : 'none';
      toggle.textContent = (hidden ? '▼' : '▶') + ' show program';
    });

    const useBtn = document.createElement('a');
    useBtn.href = '#';
    useBtn.style.cssText = 'font-size:0.7rem;color:#4ec9b0;';
    useBtn.textContent = '↑ use as baseline';
    useBtn.title = 'Copy this program to the editor for further mutations';
    useBtn.addEventListener('click', e => {
      e.preventDefault();
      programEl.value = programText;
      programEl.scrollIntoView({ behavior: 'smooth' });
    });

    actionRow.appendChild(toggle);
    actionRow.appendChild(useBtn);
    info.appendChild(actionRow);
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

function base64ToUint8Array(b64) {
  const bin = atob(b64);
  const arr = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) arr[i] = bin.charCodeAt(i);
  return arr;
}

main();

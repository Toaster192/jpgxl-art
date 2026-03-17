// Fetch the generated data from the API and render each image onto a canvas.
// The server returns raw RGBA bytes (base64), so no JXL WASM decoder is needed
// for this initial display — the canvas ImageData API accepts RGBA directly.
// JXL encode/decode is handled server-side by jpegxl-rs.

async function main() {
  const status = document.getElementById('status');
  const gallery = document.getElementById('gallery');
  const programEl = document.getElementById('program');

  let data;
  try {
    const res = await fetch('/api/generate');
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    data = await res.json();
  } catch (e) {
    status.textContent = `Error: ${e.message}`;
    return;
  }

  status.textContent = `Rendered ${1 + data.mutations.length} images.`;
  programEl.textContent = data.program_text;

  renderCard(gallery, 'Original', data.original, true);
  for (const m of data.mutations) {
    renderCard(gallery, m.label, m.image, false, m.program_text, m.warning);
  }
}

function renderCard(container, label, payload, isOriginal, programText, warning) {
  const card = document.createElement('div');
  card.className = 'card';

  const canvas = document.createElement('canvas');
  canvas.width = payload.width;
  canvas.height = payload.height;

  const rgba = base64ToUint8Array(payload.rgba_b64);
  const ctx = canvas.getContext('2d');
  const imageData = new ImageData(new Uint8ClampedArray(rgba), payload.width, payload.height);
  ctx.putImageData(imageData, 0, 0);

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

  if (warning) {
    const w = document.createElement('div');
    w.style.cssText = 'font-size:0.7rem;color:#e8a045;margin-top:4px;';
    w.textContent = '⚠ ' + warning;
    info.appendChild(w);
  }

  if (programText) {
    const toggle = document.createElement('a');
    toggle.href = '#';
    toggle.style.cssText = 'display:block;font-size:0.7rem;color:#666;margin-top:4px;';
    toggle.textContent = '▶ show program';
    const pre = document.createElement('pre');
    pre.style.cssText = 'display:none;font-size:0.65rem;color:#9cdcfe;overflow-x:auto;white-space:pre;margin-top:4px;';
    pre.textContent = programText;
    toggle.addEventListener('click', (e) => {
      e.preventDefault();
      const hidden = pre.style.display === 'none';
      pre.style.display = hidden ? 'block' : 'none';
      toggle.textContent = (hidden ? '▼' : '▶') + ' show program';
    });
    info.appendChild(toggle);
    info.appendChild(pre);
  }

  card.appendChild(canvas);
  card.appendChild(info);
  container.appendChild(card);
}

function base64ToUint8Array(b64) {
  const bin = atob(b64);
  const arr = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) arr[i] = bin.charCodeAt(i);
  return arr;
}

main();

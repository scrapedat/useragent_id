// Minimal CDP-based DOM replayer (Node.js)
// Env:
//   CDP_URL: http://127.0.0.1:9222
//   INPUT_PATH: dom_session_<session_id>.jsonl
//   SPEED_MS: optional delay between actions (default 300)
//   CAPTIONS: optional JSON array of { index:number, text:string }
//   PAUSE_FILE: optional path; if this file exists, replay pauses until removed
//   STEP_FILE: optional path; if set, advance exactly one step when this file appears (it will be deleted by replayer)
//   PROGRESS_FILE: optional path; if set, write {index,total} JSON each step
//
const CDP = require('chrome-remote-interface');
const fs = require('fs');

const CDP_URL = process.env.CDP_URL || 'http://127.0.0.1:9222';
const INPUT_PATH = process.env.INPUT_PATH;
const SPEED_MS = parseInt(process.env.SPEED_MS || '300', 10);
const CAPTIONS = (() => { try { return JSON.parse(process.env.CAPTIONS || '[]'); } catch { return []; } })();
const PAUSE_FILE = process.env.PAUSE_FILE;
const STEP_FILE = process.env.STEP_FILE;
const PROGRESS_FILE = process.env.PROGRESS_FILE;
const STOP_FILE = process.env.STOP_FILE;
async function maybePause() {
  if (!PAUSE_FILE) return;
  while (fs.existsSync(PAUSE_FILE)) {
    await sleep(200);
  }
}

async function waitForStepSignal() {
  if (!STEP_FILE) return; // step mode off
  // Wait until a step signal file exists, then consume it and proceed one step
  while (!fs.existsSync(STEP_FILE)) {
    await sleep(100);
  }
  try { fs.unlinkSync(STEP_FILE); } catch {}
}

if (!INPUT_PATH) {
  console.error('INPUT_PATH env var is required');
  process.exit(2);
}

function readLines(path) {
  return fs.readFileSync(path, 'utf8').split(/\r?\n/).filter(Boolean).map(l=>{
    try { return JSON.parse(l); } catch { return null; }
  }).filter(Boolean);
}

function sleep(ms){ return new Promise(r=>setTimeout(r, ms)); }

async function queryAndClick(Runtime, css, xpath) {
  if (css) {
    const expr = `(() => { const el = document.querySelector(${JSON.stringify(css)}); if (el) { el.scrollIntoView({block:'center'}); el.click(); return true;} return false; })()`;
    const res = await Runtime.evaluate({ expression: expr, awaitPromise: true });
    if (res.result && res.result.value) return true;
  }
  if (xpath) {
    const expr = `(() => { const r = document.evaluate(${JSON.stringify(xpath)}, document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null); const el = r.singleNodeValue; if (el) { el.scrollIntoView({block:'center'}); el.click(); return true;} return false; })()`;
    const res = await Runtime.evaluate({ expression: expr, awaitPromise: true });
    if (res.result && res.result.value) return true;
  }
  return false;
}

(async function main(){
  try {
    const url = new URL(CDP_URL);
    const targets = await CDP.List({ host: url.hostname, port: Number(url.port) });
    const pageTarget = targets.find(t => t.type === 'page') || targets[0];
    const client = await CDP({ target: pageTarget, host: url.hostname, port: Number(url.port) });
  const { Page, Runtime, Input } = client;

    await Page.enable();
    await Runtime.enable();

    const lines = readLines(INPUT_PATH);
    console.log(`Replaying ${lines.length} DOM lines from ${INPUT_PATH}`);

    // Emit initial progress so UI can show 0/N before first step
    if (PROGRESS_FILE) {
      try { fs.writeFileSync(PROGRESS_FILE, JSON.stringify({ index: 0, total: lines.length })); } catch {}
    }

    for (let i = 0; i < lines.length; i++) {
      // Allow hard stop
      if (STOP_FILE && fs.existsSync(STOP_FILE)) {
        if (PROGRESS_FILE) {
          try { fs.writeFileSync(PROGRESS_FILE, JSON.stringify({ index: i, total: lines.length })); } catch {}
        }
        break;
      }
      const line = lines[i];
  const type = line.type;
      // Progress: write index/total
      if (PROGRESS_FILE) {
        try { fs.writeFileSync(PROGRESS_FILE, JSON.stringify({ index: i, total: lines.length })); } catch {}
      }
      // Inject caption if any is assigned to this index
      const caption = CAPTIONS.find(c => c.index === i);
      if (caption) {
        const expr = `(() => { let el = document.getElementById('__wam_caption'); if (!el) { el = document.createElement('div'); el.id='__wam_caption'; Object.assign(el.style,{position:'fixed',bottom:'10px',left:'10px',right:'10px',padding:'8px 12px',background:'rgba(0,0,0,0.7)',color:'#fff',font:'14px sans-serif',borderRadius:'6px',zIndex:2147483647}); document.body.appendChild(el);} el.textContent=${JSON.stringify(caption.text)}; })()`;
        await Runtime.evaluate({ expression: expr });
      }
      await maybePause();
      // Check stop after pause too
      if (STOP_FILE && fs.existsSync(STOP_FILE)) {
        if (PROGRESS_FILE) {
          try { fs.writeFileSync(PROGRESS_FILE, JSON.stringify({ index: i, total: lines.length })); } catch {}
        }
        break;
      }
      await waitForStepSignal();
      const payload = line.payload || {};
      if (type === 'navigated' && payload.url) {
        await Page.navigate({ url: payload.url });
        await sleep(SPEED_MS);
      } else if (type === 'page_load') {
        // Light wait to allow content to settle
        await sleep(SPEED_MS);
      } else if (type === 'dom_event' && payload.kind === 'click') {
        const ok = await queryAndClick(Runtime, payload.css, payload.xpath);
        if (!ok) console.warn('Click target not found', { css: payload.css, xpath: payload.xpath });
        await sleep(SPEED_MS);
      } else if (type === 'dom_event' && payload.kind === 'keydown') {
        const key = payload.key;
        if (key === 'Enter') {
          await Input.dispatchKeyEvent({ type: 'keyDown', key: 'Enter', code: 'Enter' });
          await Input.dispatchKeyEvent({ type: 'keyUp', key: 'Enter', code: 'Enter' });
        } else if (typeof key === 'string' && key.length === 1) {
          await Input.insertText({ text: key });
        } else if (key === ' ') {
          await Input.insertText({ text: ' ' });
        }
        await sleep(SPEED_MS);
      }
    }

    // Remove caption overlay at the end
    try {
      await Runtime.evaluate({ expression: `(() => { const el = document.getElementById('__wam_caption'); if (el) el.remove(); })()` });
    } catch {}
    if (PROGRESS_FILE) {
      try { fs.writeFileSync(PROGRESS_FILE, JSON.stringify({ index: lines.length, total: lines.length })); } catch {}
    }
    await client.close();
  } catch (e) {
    console.error('Replay error:', e);
    process.exit(1);
  }
})();

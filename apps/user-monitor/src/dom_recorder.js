// Minimal CDP-based DOM recorder (Node.js)
// Requires: node >= 16, chrome-remote-interface
// Environment:
//   CDP_URL: e.g., http://127.0.0.1:9222
//   OUTPUT_PATH: file to append JSONL events
//   SESSION_ID: uuid string

const CDP = require('chrome-remote-interface');
const fs = require('fs');

const CDP_URL = process.env.CDP_URL || 'http://127.0.0.1:9222';
const OUTPUT_PATH = process.env.OUTPUT_PATH || './dom_events.jsonl';
const SESSION_ID = process.env.SESSION_ID || '00000000-0000-0000-0000-000000000000';

function writeEvent(obj) {
  fs.appendFileSync(OUTPUT_PATH, JSON.stringify(obj) + '\n');
}

(async function main() {
  try {
    const targets = await CDP.List({ host: '127.0.0.1', port: 9222 });
    const pageTarget = targets.find(t => t.type === 'page') || targets[0];
    const client = await CDP({ target: pageTarget, host: '127.0.0.1', port: 9222 });
    const { Page, DOM, Runtime, Input } = client;

    await Page.enable();
    await DOM.enable();
    await Runtime.enable();

    function log(type, payload) {
      writeEvent({ session_id: SESSION_ID, timestamp: new Date().toISOString(), type, payload });
    }

    Page.loadEventFired(async () => {
      const title = await Runtime.evaluate({ expression: 'document.title' });
      log('page_load', { title: title?.result?.value });
    });
    Page.frameNavigated(async params => {
      const url = params?.frame?.url;
      const title = await Runtime.evaluate({ expression: 'document.title' });
      log('navigated', { url, title: title?.result?.value });
    });

    // Inject a small script to capture clicks and keypresses
    const inject = `
      (() => {
        function cssSelector(el) {
          try {
            if (!el || el.nodeType !== Node.ELEMENT_NODE) return null;
            if (el.id) return '#' + CSS.escape(el.id);
            const tag = el.tagName ? el.tagName.toLowerCase() : '';
            const cls = Array.from(el.classList || []).filter(Boolean).slice(0, 2).map(c => '.' + CSS.escape(c)).join('');
            if (cls) return tag + cls;
            // fallback to nth-of-type chain (depth up to 4)
            let cur = el, parts = [], depth = 0;
            while (cur && cur.nodeType === Node.ELEMENT_NODE && depth < 4) {
              const t = cur.tagName ? cur.tagName.toLowerCase() : '*';
              const siblings = Array.from(cur.parentNode ? cur.parentNode.children : []);
              const sameTag = siblings.filter(n => n.tagName === cur.tagName);
              const idx = sameTag.indexOf(cur) + 1;
              parts.unshift(t + ':nth-of-type(' + idx + ')');
              cur = cur.parentElement;
              depth++;
            }
            return parts.length ? parts.join(' > ') : null;
          } catch (_) { return null; }
        }
        function xpath(el){
          if (el && el.nodeType === Node.ELEMENT_NODE) {
            const idx = Array.from(el.parentNode ? el.parentNode.children : []).filter(n => n.tagName === el.tagName).indexOf(el) + 1;
            return (xpath(el.parentNode) + '/' + el.tagName + '[' + idx + ']');
          }
          return '';
        }
        document.addEventListener('click', (e) => {
          const el = e.target;
          const payload = {
            kind: 'click',
            tag: el.tagName,
            xpath: xpath(el),
            css: cssSelector(el),
            id: el.id || null,
            classes: el.className || null,
            name: (el.getAttribute && el.getAttribute('name')) || null,
            ariaLabel: (el.getAttribute && el.getAttribute('aria-label')) || null,
            text: (el.innerText||'').slice(0,120)
          };
          window._wam_log && window._wam_log(payload);
        }, true);
        document.addEventListener('keydown', (e) => {
          window._wam_log && window._wam_log({ kind: 'keydown', key: e.key });
        }, true);
      })();
    `;

    await Runtime.addBinding({ name: '_wam_log' });
    Runtime.bindingCalled(({ payload }) => {
      try { log('dom_event', JSON.parse(payload)); } catch {}
    });
    await Runtime.evaluate({ expression: inject, includeCommandLineAPI: true });

    process.stdin.resume();
  } catch (e) {
    writeEvent({ session_id: SESSION_ID, timestamp: new Date().toISOString(), type: 'error', error: String(e) });
    process.exit(1);
  }
})();

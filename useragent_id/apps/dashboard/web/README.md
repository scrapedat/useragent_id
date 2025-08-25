# Dashboard Web (Public Site)

This folder contains the public-facing landing/prototype built with Vite + React + Tailwind.

## Run locally

```bash
cd useragent_id/apps/dashboard/web
npm install
npm run dev
```

## Build

```bash
npm run build
# output in dist/
```

## Deploy to GitHub Pages

This repo includes a GitHub Actions workflow that builds and deploys `dist/` to Pages on each push to main/master.

Steps:
- In GitHub → Settings → Pages, set “Source” to GitHub Actions.
- Optionally set a custom domain there. To publish a CNAME file with the site, create `public/CNAME` under this folder.

## Lead capture (investor/demo requests)

The landing page includes a subtle lead form in the “Request a demo” section.

Two options:

1) Mailto fallback (default):
   - No server required. Submits open the user’s email client with a prefilled message to `founder@useragent.id`.

2) Server endpoint (recommended):
   - Provide an HTTPS endpoint that accepts POST JSON with fields `{ name, email, company, note, href, ua, ts }`.
   - Set a repository secret named `VITE_LEADS_ENDPOINT` to your endpoint URL. The Pages workflow injects it at build time.
   - CORS: Allow the Pages domain origin.
   - Spam: Honor the honeypot field `website` (server should discard submissions where `website` is non-empty).

Example minimal handler (Cloudflare Workers):

```js
// worker.js
export default {
  async fetch(request, env, ctx) {
    if (request.method !== 'POST') return new Response('OK');
    const data = await request.json().catch(() => ({}));
    if (data.website) return new Response('ok', { status: 200 }); // honeypot
    // TODO: store in your CRM or send email
    // Example: console.log or env.LEADS_KV.put(...)
    return new Response('ok', {
      headers: { 'Access-Control-Allow-Origin': '*', 'Content-Type': 'application/json' }
    });
  }
}
```

Security tips:
- Validate and rate-limit server-side.
- Never expose secrets in client code; only pass a public HTTPS endpoint via the `VITE_LEADS_ENDPOINT` env.

## Tailwind & VS Code

If VS Code shows warnings for `@tailwind` in CSS, the workspace settings already point the Tailwind extension to `web/tailwind.config.js` and ignore unknown at-rules.

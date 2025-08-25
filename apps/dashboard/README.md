# Dashboard App

This app contains:

- A Rust eframe/egui desktop dashboard (compiled via Cargo).
- An optional React + Vite web UI (in `web/`) for rapid prototyping.

## Run the Rust dashboard

Use the standard workspace build/run:

- cargo run -p dashboard

## Run the web UI (prototype)

From this folder:

- npm install
- npm run dev

The web app entry is `web/index.html`, sourcing files from `web/src/` (Tailwind enabled).

## Notes

- The Rust dashboard and the web UI are separate; they do not share runtime.

### Handling leads (investor/demo requests)

- The web landing includes a lead form with a mailto fallback and an optional server endpoint.
- To enable a server endpoint on Pages builds, add a repo secret `VITE_LEADS_ENDPOINT` with your HTTPS URL.
- See `apps/dashboard/web/README.md` for a minimal server example and security tips.
## Deploy as a website (GitHub Pages)

This repo ships with a workflow to deploy the dashboard web UI to GitHub Pages.

What it does:

- Builds `apps/dashboard/web/` with Vite
- Uploads the `dist/` as a Pages artifact
- Deploys to the `github-pages` environment

How to use:

1. Push to `main` (or run the workflow manually in Actions).
2. In GitHub → Settings → Pages, ensure Source is set to “GitHub Actions”.
3. The site will be published at `https://<org>.github.io/<repo>/`.

Custom domain:

- In GitHub → Settings → Pages, add your domain under “Custom domain”.
- Optional: add a `CNAME` file under `apps/dashboard/web/public/` with the domain; it will be included in builds.

Notes on backend:

- GitHub Pages is static hosting. Features that require the local recorder/replayer (Chromium CDP, Node, or Rust apps) won’t run in the hosted site. The hosted UI is great for demos, screenshots, and product narrative; automation features require running locally.

## Session Replay Controls

- In the Sessions tab, use Replay DOM to launch the Node-based replayer.
- Speed slider: adjusts delay per action (50–1200 ms).
- Pause/Resume: toggles a temp file that the replayer watches to pause execution.
- Step Mode: advance one recorded DOM action at a time with a “Step” signal.
- Stop: end the replay early with a stop signal.

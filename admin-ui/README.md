# T-Cube Admin UI

This directory contains the Svelte + Vite source for the static parent/admin dashboard.

The UI styling system uses Tailwind CSS v4 through Vite plus local CSS layers in `src/styles.css`. Keep shared visual primitives in Svelte components under `src/components/` and keep API calls in `src/api.ts`.

Use `pnpm` for every admin UI and JavaScript workflow.

```sh
pnpm --dir admin-ui install
pnpm --dir admin-ui run build
pnpm --dir admin-ui run check
```

`pnpm --dir admin-ui run build` writes deployable static files to `admin-ui/build/`. The Raspberry Pi serves only `admin-ui/build/` through `tcube-pi-admin`; it does not run Node or pnpm.

# Pace Buddy Frontend

React + TypeScript + Vite frontend for Pace Buddy.

## Development

```bash
npm install
npm run dev
```

The development server proxies `/api` to `http://localhost:8080`; run the Rust backend separately.

## Scripts

- `npm run dev`: start Vite with hot reload.
- `npm run build`: type-check and build the production bundle.
- `npm test`: run the current frontend test command, which is a production build.
- `npm run lint`: run ESLint.
- `npm run preview`: preview the built app locally.

## Notes

The frontend does not own authentication state directly. It relies on backend Strava OAuth routes and the session cookie
set by the API server.

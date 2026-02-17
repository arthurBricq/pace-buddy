# ---------- Stage 1: Build frontend ----------
FROM node:22-alpine AS frontend-build
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build


# ---------- Stage 2: Build backend ----------
FROM rust:1.93-bookworm AS backend-build
WORKDIR /app/backend
COPY backend/ ./
RUN cargo build --release -p bin


# ---------- Stage 3: Runtime ----------
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=backend-build /app/backend/target/release/bin /usr/local/bin/running-tool
COPY --from=frontend-build /app/frontend/dist /srv/frontend

ENV HOST=0.0.0.0
ENV PORT=8080

EXPOSE 8080

CMD ["running-tool", "--static-serving", "/srv/frontend"]

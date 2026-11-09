# <app> API image: two-stage node:24-alpine build — node dist/bin/{api,worker}.js
# (server + maintenance loops); migrations ride the node-pg-migrate CLI that is
# already in node_modules. The template ships with literal placeholders and no
# pnpm-lock.yaml (factory invariant INV-T3): the build RUN self-heals with
# `pnpm install` when pnpm-lock.yaml is missing, so a fresh clone + rename.sh
# composes immediately; once a committed lockfile exists, --frozen-lockfile
# keeps the install reproducible.

FROM node:24-alpine AS build
WORKDIR /src
COPY package.json pnpm-lock.yaml* ./
RUN --mount=type=cache,target=/root/.cache/pnpm \
    corepack enable \
        && if [ -f pnpm-lock.yaml ]; then pnpm install --frozen-lockfile; else pnpm install; fi
COPY . .
RUN --mount=type=cache,target=/root/.cache/pnpm \
    pnpm build

FROM node:24-alpine
WORKDIR /app
ENV NODE_ENV=production
COPY --from=build /src/dist ./dist
COPY --from=build /src/node_modules ./node_modules
COPY --from=build /src/package.json ./
# tsc never rewrites package "imports" on emit — the compiled dist/
# re-points the subpath-import map onto the dist tree (source layout is
# mirrored into dist/, so #platform/X.js resolves to dist/platform/X.js).
RUN node -e "const fs=require('node:fs');const p=JSON.parse(fs.readFileSync('package.json','utf8'));for (const k of Object.keys(p.imports)) p.imports[k]='./dist'+p.imports[k].slice(5);fs.writeFileSync('package.json', JSON.stringify(p))"
USER node
EXPOSE 8080
# Healthcheck probe mode: the api process GETs its own /healthz and exits
# non-zero on failure (the runtime image carries no shell-based curl).
HEALTHCHECK --interval=15s --timeout=3s --start-period=10s --retries=3 \
  CMD ["node", "dist/bin/api.js", "probe"]
ENTRYPOINT ["node", "dist/bin/api.js"]

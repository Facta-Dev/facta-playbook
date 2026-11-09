# syntax=docker/dockerfile:1
# `<app>` edge nginx — reverse-proxies the api container inside the compose
# network. TLS terminates upstream (ALB / Cloudflare); this serves :80.
# Build context = ./infra/nginx (see docker-compose.prod.yml).
# ponytail: rename every `<app>` -> your project name before building. The conf
# FILENAME is fixed (default.conf) so rename.sh's crates/<app>-* dir rename
# does not need to touch it; only the conf CONTENT carries <app> placeholders.
FROM nginx:1.27-alpine
RUN rm -f /etc/nginx/conf.d/default.conf
COPY default.conf /etc/nginx/conf.d/default.conf
RUN apk add --no-cache wget
HEALTHCHECK --interval=15s --timeout=5s --retries=5 \
    CMD nginx -t && wget -q -O /dev/null http://127.0.0.1/healthz || exit 1

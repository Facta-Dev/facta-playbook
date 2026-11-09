# <app> API image: static Go binaries (CGO_ENABLED=0) on distroless —
# /api (server), /worker (janitor + outbox), /migrate (goose release step).
# The template ships with literal placeholders and no go.sum (factory
# invariant INV-T3): the build RUN self-heals with `go mod tidy` when go.sum
# is missing, so a fresh clone + rename.sh composes immediately; once a
# committed go.sum exists, -mod=readonly keeps the build reproducible.
FROM golang:1.27-alpine AS build
WORKDIR /src
ENV CGO_ENABLED=0 GOFLAGS=-mod=readonly
COPY go.mod go.sum* ./
COPY . .
RUN --mount=type=cache,target=/go/pkg/mod --mount=type=cache,target=/root/.cache/go-build \
    test -f go.sum || go mod tidy \
    && go build -trimpath -ldflags "-s -w" -o /out/api ./cmd/api \
    && go build -trimpath -ldflags "-s -w" -o /out/worker ./cmd/worker \
    && go build -trimpath -ldflags "-s -w" -o /out/migrate ./cmd/migrate

FROM gcr.io/distroless/static-debian12:nonroot
COPY --from=build /out/api /api
COPY --from=build /out/worker /worker
COPY --from=build /out/migrate /migrate
USER nonroot:nonroot
EXPOSE 8080
ENTRYPOINT ["/api"]
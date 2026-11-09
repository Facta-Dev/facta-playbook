# <app> API image: single-stage python:3.12-slim. The template ships without
# a lockfile (factory invariant INV-T3), so the build installs the project
# from pyproject.toml ranges; once an instantiated repo commits uv.lock,
# switch the install to `uv sync --frozen` if you want byte-identical builds.
FROM python:3.12-slim

WORKDIR /app
ENV PYTHONUNBUFFERED=1 \
    PIP_NO_CACHE_DIR=1 \
    PIP_DISABLE_PIP_VERSION_CHECK=1

# pyproject reads README.md; src/ is the only package source.
COPY pyproject.toml README.md ./
COPY src ./src
RUN pip install .

# Alembic is the deploy step (compose `migrations` service), never the API boot.
COPY alembic.ini ./
COPY migrations ./migrations

RUN useradd --create-home --uid 10001 app && chown -R app /app
USER app

EXPOSE 8000
# stdlib probe: the runtime image carries no curl.
HEALTHCHECK --interval=15s --timeout=3s --start-period=10s --retries=3 \
  CMD ["python", "-c", "import urllib.request; urllib.request.urlopen('http://127.0.0.1:8000/healthz', timeout=3)"]

CMD ["python", "-m", "app.bin.api"]

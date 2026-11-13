# Realtime and streaming

Realtime is optional. The template does not ship an SSE/WebSocket adapter (known gap): add one behind a port if the product needs it, with snapshot + stream, dedupe by event id, resume and a polling fallback. JMS is for durable work, not for UI updates.

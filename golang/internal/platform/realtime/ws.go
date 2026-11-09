package realtime

import (
	"context"
	"io"
	"sync"
	"time"

	"github.com/coder/websocket"
	"github.com/coder/websocket/wsjson"
)

// Frame is the wire shape in both directions: {"type", "payload"} (ctx0).
type Frame struct {
	Type    string `json:"type"`
	Payload any    `json:"payload,omitempty"`
}

// Keepalive interval and the ctx0 close semantics for failed authentication.
const (
	pingPeriod          = 30 * time.Second
	pingTimeout         = 10 * time.Second
	authFailedCloseCode = 4001
)

// CloseAuthFailed rejects a connection whose token failed verification; call
// only after websocket.Accept (close codes are meaningless pre-handshake).
func CloseAuthFailed(conn *websocket.Conn) {
	_ = conn.Close(websocket.StatusCode(authFailedCloseCode), "authentication_failed")
}

// ServeConn pumps one authenticated connection: discards client frames, pings
// every 30s and streams hub events as JSON frames. It returns on peer
// disconnect, context end, or write/ping failure; the caller conn.CloseNow()s.
func ServeConn(ctx context.Context, conn *websocket.Conn, ch <-chan Event) {
	// coder/websocket allows one concurrent reader and one concurrent writer;
	// the ping and frame writes share wmu, the read loop runs in its goroutine.
	var wmu sync.Mutex
	ctx, cancel := context.WithCancel(ctx)
	defer cancel()

	go func() {
		defer cancel()
		for {
			_, reader, err := conn.Reader(ctx)
			if err != nil {
				return
			}
			if _, err := io.Copy(io.Discard, reader); err != nil {
				return
			}
		}
	}()

	ticker := time.NewTicker(pingPeriod)
	defer ticker.Stop()
	for {
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
			pctx, pcancel := context.WithTimeout(ctx, pingTimeout)
			wmu.Lock()
			err := conn.Ping(pctx)
			wmu.Unlock()
			pcancel()
			if err != nil {
				return
			}
		case ev, ok := <-ch:
			if !ok {
				return
			}
			wmu.Lock()
			err := wsjson.Write(ctx, conn, Frame{Type: ev.Type, Payload: ev.Payload})
			wmu.Unlock()
			if err != nil {
				return
			}
		}
	}
}

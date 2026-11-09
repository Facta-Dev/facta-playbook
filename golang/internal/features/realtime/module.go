// Package realtime owns the WS endpoint: one GET /api/v1/ws route that
// upgrades, authenticates via ?token=, subscribes to the caller's identity
// room and streams platform hub events as JSON frames.
package realtime

import (
	"net/http"

	"github.com/coder/websocket"

	"github.com/<scope>/<app>/internal/platform/authx"
	platrt "github.com/<scope>/<app>/internal/platform/realtime"
)

// Verifier is the JWT parse port (platform/authx satisfies it).
type Verifier interface {
	Parse(raw string) (authx.Identity, error)
}

// Publisher is the hub port for subscribing to the identity room.
type Publisher interface {
	Subscribe(room string) (<-chan platrt.Event, func())
}

// Deps carries everything Register needs; main.go is the only caller.
type Deps struct {
	Verifier  Verifier
	Publisher Publisher
}

// Register mounts the WS route.
func Register(mux *http.ServeMux, d Deps) {
	h := handlers{v: d.Verifier, pub: d.Publisher}
	mux.Handle("GET /api/v1/ws", http.HandlerFunc(h.ws))
}

type handlers struct {
	v   Verifier
	pub Publisher
}

// ws upgrades first, then authenticates: bad tokens get close 4001
// ("authentication_failed") so clients can react programmatically.
func (h handlers) ws(w http.ResponseWriter, r *http.Request) {
	conn, err := websocket.Accept(w, r, nil)
	if err != nil {
		return
	}
	defer func() { _ = conn.CloseNow() }()

	id, err := h.v.Parse(r.URL.Query().Get("token"))
	if err != nil {
		platrt.CloseAuthFailed(conn)
		return
	}
	ch, cancel := h.pub.Subscribe(platrt.RoomUser(id.UserID))
	defer cancel()

	platrt.ServeConn(r.Context(), conn, ch)
	_ = conn.Close(websocket.StatusNormalClosure, "")
}

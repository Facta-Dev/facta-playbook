package widgets

import (
	"net/http"

	"github.com/<scope>/<app>/internal/platform/authx"
)

// Deps carries everything Register needs; main.go is the only caller.
type Deps struct {
	Store      Store
	Publisher  Publisher
	Verifier   authx.Verifier
	CookieName string // forwarded to RequireAuth
}

// Register mounts the slice's routes; everything sits behind RequireAuth.
func Register(mux *http.ServeMux, d Deps) {
	uc := New(d.Store, d.Publisher, nil)
	h := handlers{uc: uc, pub: d.Publisher}
	auth := authx.RequireAuth(d.Verifier, d.CookieName)
	mux.Handle("GET /api/v1/widgets", auth(http.HandlerFunc(h.list)))
	mux.Handle("POST /api/v1/widgets", auth(http.HandlerFunc(h.create)))
	mux.Handle("GET /api/v1/widgets/events", auth(http.HandlerFunc(h.events)))
	mux.Handle("GET /api/v1/widgets/{id}", auth(http.HandlerFunc(h.get)))
	mux.Handle("PUT /api/v1/widgets/{id}", auth(http.HandlerFunc(h.update)))
	mux.Handle("DELETE /api/v1/widgets/{id}", auth(http.HandlerFunc(h.delete)))
}

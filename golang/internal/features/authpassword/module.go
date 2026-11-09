package authpassword

import (
	"net/http"
	"time"

	"github.com/<scope>/<app>/internal/platform/authx"
	"github.com/<scope>/<app>/internal/shared/clockx"
)

// Deps carries everything Register needs; main.go is the only caller.
type Deps struct {
	Store            Store
	Emails           EmailEnqueuer
	Verifier         authx.Verifier
	Clock            clockx.Clock
	Pepper           string
	CookieName       string // forwarded to RequireAuth
	ResetTTL         time.Duration
	RateLimit        int // max codes per window
	RateWindow       time.Duration
	LogOutAllOnReset bool
}

// Register mounts the slice's routes; change-password sits behind RequireAuth.
func Register(mux *http.ServeMux, d Deps) {
	uc := New(d.Store, d.Emails, d.Clock, d.Pepper, d.ResetTTL, d.RateWindow, d.RateLimit, d.LogOutAllOnReset)
	h := handlers{uc: uc}
	mux.HandleFunc("POST /api/v1/auth/forgot-password", h.forgot)
	mux.HandleFunc("POST /api/v1/auth/validate-code", h.validate)
	mux.HandleFunc("POST /api/v1/auth/reset-password", h.reset)
	mux.Handle("POST /api/v1/auth/change-password", authx.RequireAuth(d.Verifier, d.CookieName)(http.HandlerFunc(h.change)))
}

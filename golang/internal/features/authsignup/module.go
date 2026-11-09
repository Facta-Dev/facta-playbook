package authsignup

import (
	"net/http"
	"time"

	"github.com/<scope>/<app>/internal/shared/clockx"
)

// Deps carries everything Register needs; main.go is the only caller.
type Deps struct {
	Store     Store
	Emails    EmailEnqueuer
	Clock     clockx.Clock
	Pepper    string
	VerifyTTL time.Duration
}

// Register mounts the slice's routes on the shared mux.
func Register(mux *http.ServeMux, d Deps) {
	uc := New(d.Store, d.Emails, d.Clock, d.Pepper, d.VerifyTTL)
	h := handlers{uc: uc}
	mux.HandleFunc("POST /api/v1/auth/signup", h.signup)
	mux.HandleFunc("POST /api/v1/auth/verify", h.verifyEmail)
	mux.HandleFunc("POST /api/v1/auth/verify/resend", h.resend)
}

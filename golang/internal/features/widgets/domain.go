// Package widgets is the example CRUD domain, owner-scoped: every read and
// write is filtered by the authenticated owner, and lifecycle changes are
// published to the realtime hub (widget.created / updated / deleted).
package widgets

import (
	"time"

	"github.com/<scope>/<app>/internal/shared/apperr"
)

// Widget is one owned row.
type Widget struct {
	ID          string
	Name        string
	Kind        string
	OwnerUserID string
	CreatedAt   time.Time
}

var (
	ErrNotFound    = apperr.E(apperr.CodeNotFound, apperr.NotFound, "widget not found")
	ErrForbidden   = apperr.E(apperr.CodeForbidden, apperr.AuthForbidden, "not your widget")
	ErrValidation  = apperr.E(apperr.CodeUnprocessable, apperr.Validation, "invalid request")
	ErrUnknownKind = apperr.E(apperr.CodeUnprocessable, apperr.Validation, "unknown kind")
)

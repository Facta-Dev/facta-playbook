// Package authz answers the one authorization question the auth slices ask:
// may this actor manage that account — itself, or a subordinate in the rolex
// hierarchy? ctx0 parity: CanManageSubordinate = Rank(subject) > Rank(target);
// CanManageSelf = same id AND Rank(target) <= Rank(subject) — equals never
// manage equals, and a stale token cannot outrank the row it acts on.
package authz

import (
	"github.com/<scope>/<app>/internal/shared/apperr"
	"github.com/<scope>/<app>/internal/shared/rolex"
)

// ErrForbidden is the stable 403 guards raise when the answer is no.
var ErrForbidden = apperr.E(apperr.CodeForbidden, apperr.AuthForbidden, "insufficient role")

// Context is one authorization question: Subject (the actor, id + role from
// the verified token) acts on Target (id + role from the store).
type Context struct {
	SubjectID   string
	SubjectRole rolex.Role
	TargetID    string
	TargetRole  rolex.Role
}

// CanManageSelf reports whether the actor IS the target and may keep
// managing it: the acting role must still rank at-or-above the target's DB
// role — a token minted before a demotion answers false.
func (c Context) CanManageSelf() bool {
	return c.SubjectID != "" && c.SubjectID == c.TargetID &&
		rolex.Rank(c.TargetRole) <= rolex.Rank(c.SubjectRole)
}

// CanManageSubordinate reports whether the actor outranks the target. The
// hierarchy is strict: an ADMIN cannot reach another ADMIN's row.
func (c Context) CanManageSubordinate() bool {
	return rolex.Rank(c.SubjectRole) > rolex.Rank(c.TargetRole)
}

// CanManage is the composite use-case rule: self or subordinate.
func (c Context) CanManage() bool {
	return c.CanManageSelf() || c.CanManageSubordinate()
}

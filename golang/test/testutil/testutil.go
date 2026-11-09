// Package testutil holds the tiny doubles shared by the test suites: a
// clock the harness controls, a recorded email enqueuer, a real (HS256)
// JWT minter/verifier pair — acceptance mints live tokens, it does not fake
// cryptography — and a deterministic, UUID-shaped id sequence.
package testutil

import (
	"context"
	"fmt"
	"sync"
	"time"

	"github.com/<scope>/<app>/internal/platform/authx"
	"github.com/<scope>/<app>/internal/platform/email"
	"github.com/<scope>/<app>/internal/shared/clockx"
)

// Shared test credentials; TestIssuer matches main.go's issuer placeholder
// so the same rename substitution keeps verifier and minter aligned.
const (
	TestJWTSecret = "test-secret-32-bytes-0123456789abcedf"
	TestPepper    = "test-pepper-32-bytes-0123456789abcdef"
	TestIssuer    = "<app>-api"
)

// NewMinter builds the real HS256 minter used by every test suite.
func NewMinter() *authx.HS256Minter { return authx.NewHS256Minter(TestJWTSecret, TestIssuer) }

// NewVerifier builds the verifier matching NewMinter.
func NewVerifier() *authx.HS256Verifier { return authx.NewHS256Verifier(TestJWTSecret, TestIssuer) }

// TestClock is a clockx.Clock the harness controls; "N hours pass" steps
// advance it so TTL expiry is deterministic.
type TestClock struct {
	mu  sync.Mutex
	now time.Time
}

var _ clockx.Clock = (*TestClock)(nil)

// NewClock starts the scenario at a fixed epoch.
func NewClock() *TestClock {
	return &TestClock{now: time.Date(2026, 1, 1, 10, 0, 0, 0, time.UTC)}
}

// Now implements clockx.Clock.
func (c *TestClock) Now() time.Time {
	c.mu.Lock()
	defer c.mu.Unlock()
	return c.now
}

// Advance moves the clock forward by d.
func (c *TestClock) Advance(d time.Duration) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.now = c.now.Add(d)
}

// RecordedEmails captures what slices enqueue onto the outbox port. Steps
// count templates per recipient and read the emailed 6-digit codes.
type RecordedEmails struct {
	mu   sync.Mutex
	sent []email.Message
}

// NewEmails returns an empty recorder implementing every slice's
// EmailEnqueuer port structurally.
func NewEmails() *RecordedEmails { return &RecordedEmails{} }

// Enqueue records the message (always succeeds).
func (r *RecordedEmails) Enqueue(_ context.Context, ms email.Message) error {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.sent = append(r.sent, ms)
	return nil
}

// Sent returns a copy of everything enqueued so far.
func (r *RecordedEmails) Sent() []email.Message {
	r.mu.Lock()
	defer r.mu.Unlock()
	out := make([]email.Message, len(r.sent))
	copy(out, r.sent)
	return out
}

// Count returns how many messages with this template went to this address.
func (r *RecordedEmails) Count(to, templateKey string) int {
	r.mu.Lock()
	defer r.mu.Unlock()
	n := 0
	for _, ms := range r.sent {
		if ms.To == to && ms.TemplateKey == templateKey {
			n++
		}
	}
	return n
}

// LatestCode returns the 6-digit code of the newest matching message.
func (r *RecordedEmails) LatestCode(to, templateKey string) (string, bool) {
	r.mu.Lock()
	defer r.mu.Unlock()
	for i := len(r.sent) - 1; i >= 0; i-- {
		ms := r.sent[i]
		if ms.To == to && ms.TemplateKey == templateKey {
			code, _ := ms.Vars["code"].(string)
			return code, true
		}
	}
	return "", false
}

var idSeq int

// ResetIDs restarts the deterministic id sequence; every godog scenario
// seeds from …001, so feature texts can reference literal ids.
func ResetIDs() { idSeq = 0 }

// NextID returns the next deterministic UUID-shaped id (postgres UUID
// columns and uuid.Parse need the shape). Scenarios are serial, so the
// counter needs no lock.
func NextID() string {
	idSeq++
	return fmt.Sprintf("00000000-0000-0000-0000-%012d", idSeq)
}

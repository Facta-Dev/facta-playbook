package widgets

import (
	"context"

	"github.com/<scope>/<app>/internal/platform/realtime"
)

// Store persists owner-scoped widgets (impl: postgres.go).
type Store interface {
	Insert(ctx context.Context, w Widget) error
	List(ctx context.Context, ownerID string, limit, offset int) ([]Widget, error)
	Get(ctx context.Context, id, ownerID string) (Widget, bool, error)
	Update(ctx context.Context, id, ownerID string, name, kind string) error
	Delete(ctx context.Context, id, ownerID string) error
}

// Publisher is the realtime hub port: publish lifecycle events and hand out
// per-subscriber channels for the SSE stream.
type Publisher interface {
	Publish(ev realtime.Event)
	Subscribe(room string) (<-chan realtime.Event, func())
}

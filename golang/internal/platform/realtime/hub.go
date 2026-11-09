// Package realtime is the in-process event hub for WS/SSE delivery. ctx0
// parity: Redis pub/sub is the documented evolution path (adapter ready).
package realtime

import (
	"sync"
	"time"
)

// Event is one notification routed to a room.
type Event struct {
	Room    string // "user:<userID>" (see RoomUser) or a topic room
	Type    string
	Payload any
	At      time.Time
}

// EventBus publishes room events and hands out per-subscriber channels.
// Publish never blocks: events are dropped, not queued, when the buffer is
// full — a slow consumer must not stall a use-case goroutine.
type EventBus interface {
	Publish(ev Event)
	Subscribe(room string) (<-chan Event, func())
}

// RoomUser is the identity room vocabulary shared by WS and SSE: one room per
// user; events published there reach all that user's live connections.
func RoomUser(userID string) string { return "user:" + userID }

type hub struct {
	mu   sync.RWMutex
	buf  int
	subs map[string]map[chan Event]struct{}
}

// NewHub builds an EventBus with per-subscriber buffer (<=0 => 16).
func NewHub(buffer int) EventBus {
	if buffer <= 0 {
		buffer = 16
	}
	return &hub{buf: buffer, subs: map[string]map[chan Event]struct{}{}}
}

var _ EventBus = (*hub)(nil)

func (h *hub) Publish(ev Event) {
	h.mu.RLock()
	defer h.mu.RUnlock()
	for ch := range h.subs[ev.Room] {
		select {
		case ch <- ev:
		default: // drop on full buffer
		}
	}
}

func (h *hub) Subscribe(room string) (<-chan Event, func()) {
	ch := make(chan Event, h.buf)
	h.mu.Lock()
	set := h.subs[room]
	if set == nil {
		set = map[chan Event]struct{}{}
		h.subs[room] = set
	}
	set[ch] = struct{}{}
	h.mu.Unlock()

	var once sync.Once
	return ch, func() {
		once.Do(func() {
			h.mu.Lock()
			delete(h.subs[room], ch)
			h.mu.Unlock()
			close(ch)
		})
	}
}

package realtime

import (
	"strings"
	"testing"
	"time"
)

func TestHubSubscribePublishCancel(t *testing.T) {
	t.Parallel()
	h := NewHub(4)
	ch, cancel := h.Subscribe("user:u-1")
	defer cancel()

	ev := Event{Room: RoomUser("u-1"), Type: "widget.created", Payload: map[string]any{"id": "w-1"}}
	h.Publish(ev)
	select {
	case got := <-ch:
		if got.Type != ev.Type {
			t.Fatalf("got %+v", got)
		}
	case <-time.After(time.Second):
		t.Fatal("event not delivered")
	}
}

func TestHubRoomIsolation(t *testing.T) {
	t.Parallel()
	h := NewHub(4)
	ch, cancel := h.Subscribe("user:a")
	defer cancel()
	h.Publish(Event{Room: RoomUser("b"), Type: "nope"})
	select {
	case got := <-ch:
		t.Fatalf("cross-room leak: %+v", got)
	case <-time.After(50 * time.Millisecond):
	}
}

func TestHubDropsWhenFull(t *testing.T) {
	t.Parallel()
	h := NewHub(1)
	ch, cancel := h.Subscribe("user:a")
	defer cancel()
	h.Publish(Event{Room: "user:a", Type: "one"}) // fills the buffer
	h.Publish(Event{Room: "user:a", Type: "two"}) // must drop, not block
	h.Publish(Event{Room: "user:a", Type: "three"})
	// Publish must have returned even though the buffer holds one event.
	select {
	case <-ch:
	case <-time.After(time.Second):
		t.Fatal("queued event lost")
	}
	// The buffered channel now holds at most one event: a non-blocking drain
	// proves the other two publishes dropped instead of queueing behind it.
	deadline := time.After(50 * time.Millisecond)
	n := 0
drain:
	for {
		select {
		case <-ch:
			n++
		case <-deadline:
			break drain
		}
	}
	if n > 1 {
		t.Fatalf("buffer grew beyond capacity: %d", n)
	}
}

func TestHubCancelCloses(t *testing.T) {
	t.Parallel()
	h := NewHub(1)
	ch, cancel := h.Subscribe("user:a")
	cancel()
	if _, ok := <-ch; ok {
		t.Fatal("want closed channel after cancel")
	}
}

func TestRoomUserPrefix(t *testing.T) {
	t.Parallel()
	if !strings.HasPrefix(RoomUser("u-1"), "user:") {
		t.Fatal("identity room must be namespaced user:<id>")
	}
}

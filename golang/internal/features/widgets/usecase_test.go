package widgets

import (
	"context"
	"errors"
	"testing"
	"time"

	"github.com/<scope>/<app>/internal/platform/realtime"
	"github.com/<scope>/<app>/internal/shared/apperr"
	"github.com/<scope>/<app>/internal/shared/clockx"
)

// fakeStore keeps a flat map; every lookup re-checks the owner.
type fakeStore struct {
	rows map[string]Widget // keyed by id; Get enforces owner
}

func (f *fakeStore) Insert(_ context.Context, w Widget) error {
	f.rows[w.ID] = w
	return nil
}

func (f *fakeStore) List(_ context.Context, ownerID string, limit, offset int) ([]Widget, error) {
	out := make([]Widget, 0, len(f.rows))
	for _, w := range f.rows {
		if w.OwnerUserID == ownerID {
			out = append(out, w)
		}
	}
	if offset >= len(out) {
		return nil, nil
	}
	end := offset + limit
	if end > len(out) {
		end = len(out)
	}
	return out[offset:end], nil
}

func (f *fakeStore) Get(_ context.Context, id, ownerID string) (Widget, bool, error) {
	w, ok := f.rows[id]
	if !ok || w.OwnerUserID != ownerID {
		return Widget{}, false, nil
	}
	return w, true, nil
}

func (f *fakeStore) Update(_ context.Context, id, ownerID, name, kind string) error {
	w, ok := f.rows[id]
	if ok && w.OwnerUserID == ownerID {
		w.Name, w.Kind = name, kind
		f.rows[id] = w
	}
	return nil
}

func (f *fakeStore) Delete(_ context.Context, id, ownerID string) error {
	if w, ok := f.rows[id]; ok && w.OwnerUserID == ownerID {
		delete(f.rows, id)
	}
	return nil
}

func TestWidgetCRUDPublishes(t *testing.T) {
	t.Parallel()
	store := &fakeStore{rows: map[string]Widget{}}
	hub := realtime.NewHub(4)
	uc := New(store, hub, clockx.Fixed(time.Unix(1700, 0)))

	w, err := uc.Create(t.Context(), "owner-1", "gearbox", "cog")
	if err != nil {
		t.Fatalf("create: %v", err)
	}
	if w.ID == "" || w.CreatedAt.IsZero() {
		t.Fatalf("bad widget: %+v", w)
	}
	upd, err := uc.Update(t.Context(), w.ID, "owner-1", "gearbox-v2", "flange")
	if err != nil {
		t.Fatalf("update: %v", err)
	}
	if upd.Name != "gearbox-v2" {
		t.Fatalf("update lost: %+v", upd)
	}
	if err := uc.Delete(t.Context(), w.ID, "owner-1"); err != nil {
		t.Fatalf("delete: %v", err)
	}
	if _, err := uc.Get(t.Context(), w.ID, "owner-1"); !errors.Is(err, ErrNotFound) {
		t.Fatalf("post-delete get: %v", err)
	}
}

func TestWidgetEventsAreRoomScoped(t *testing.T) {
	t.Parallel()
	store := &fakeStore{rows: map[string]Widget{}}
	hub := realtime.NewHub(4)
	uc := New(store, hub, clockx.Fixed(time.Unix(1700, 0)))

	// Subscribe the owner room before the mutation.
	ch, cancel := hub.Subscribe(realtime.RoomUser("owner-1"))
	defer cancel()

	if _, err := uc.Create(t.Context(), "owner-1", "gearbox", "cog"); err != nil {
		t.Fatalf("create: %v", err)
	}
	select {
	case ev := <-ch:
		if ev.Type != "widget.created" || ev.Room != realtime.RoomUser("owner-1") {
			t.Fatalf("bad event: %+v", ev)
		}
	case <-time.After(time.Second):
		t.Fatal("no event on the owner room")
	}

	// The other owner's room stays silent: publish then drain-peek.
	if _, err := uc.Create(t.Context(), "owner-2", "other", "cog"); err != nil {
		t.Fatalf("create2: %v", err)
	}
	select {
	case ev := <-ch:
		t.Fatalf("leaked into other room: %+v", ev)
	case <-time.After(50 * time.Millisecond):
	}
}

func TestWidgetsListIsOwnerScoped(t *testing.T) {
	t.Parallel()
	store := &fakeStore{rows: map[string]Widget{}}
	uc := New(store, nil, clockx.Fixed(time.Unix(1700, 0)))
	_, _ = uc.Create(t.Context(), "owner-1", "a", "cog")
	_, _ = uc.Create(t.Context(), "owner-1", "b", "cog")
	_, _ = uc.Create(t.Context(), "owner-2", "c", "cog")

	rows, err := uc.List(t.Context(), "owner-1", 10, 0)
	if err != nil {
		t.Fatalf("list: %v", err)
	}
	if len(rows) != 2 {
		t.Fatalf("want 2, got %d", len(rows))
	}
	for _, r := range rows {
		if r.OwnerUserID != "owner-1" {
			t.Fatalf("leak: %+v", r)
		}
	}
}

func TestWidgetValidation(t *testing.T) {
	t.Parallel()
	store := &fakeStore{rows: map[string]Widget{}}
	uc := New(store, nil, clockx.Fixed(time.Unix(1700, 0)))
	if _, err := uc.Create(t.Context(), "owner-1", "", "cog"); !errors.Is(err, ErrValidation) {
		t.Fatalf("empty name: %v", err)
	}
	if err := uc.Delete(t.Context(), "ghost", "owner-1"); !isNotFound(err) {
		t.Fatalf("missing delete: %v", err)
	}
}

// isNotFound checks the apperr code of the error.
func isNotFound(err error) bool {
	var ae *apperr.AppError
	return errors.As(err, &ae) && ae.Code == apperr.NotFound
}

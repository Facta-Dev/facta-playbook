package widgets

import (
	"context"
	"time"

	"github.com/<scope>/<app>/internal/platform/realtime"
	"github.com/<scope>/<app>/internal/shared/clockx"
	"github.com/<scope>/<app>/internal/shared/idgen"
)

// UseCase implements the widget CRUD + event publication.
type UseCase struct {
	store Store
	pub   Publisher
	clock clockx.Clock
}

// New wires the use-case; nil clock falls back to the system clock.
func New(store Store, pub Publisher, clock clockx.Clock) *UseCase {
	if clock == nil {
		clock = clockx.System{}
	}
	return &UseCase{store: store, pub: pub, clock: clock}
}

// Create inserts a widget and publishes widget.created to the owner room.
func (uc *UseCase) Create(ctx context.Context, ownerID, name, kind string) (Widget, error) {
	if name == "" {
		return Widget{}, ErrValidation
	}
	w := Widget{
		ID: idgen.UUIDGen{}.NewID(), Name: name, Kind: kind, OwnerUserID: ownerID,
		CreatedAt: uc.clock.Now(),
	}
	if err := uc.store.Insert(ctx, w); err != nil {
		return Widget{}, err
	}
	uc.publish(ownerID, "widget.created", w)
	return w, nil
}

// List returns one page of the owner's widgets.
func (uc *UseCase) List(ctx context.Context, ownerID string, limit, offset int) ([]Widget, error) {
	return uc.store.List(ctx, ownerID, limit, offset)
}

// Get fetches one of the owner's widgets.
func (uc *UseCase) Get(ctx context.Context, id, ownerID string) (Widget, error) {
	w, ok, err := uc.store.Get(ctx, id, ownerID)
	if err != nil {
		return Widget{}, err
	}
	if !ok {
		return Widget{}, ErrNotFound
	}
	return w, nil
}

// Update renames/re-kinds a widget and publishes widget.updated.
func (uc *UseCase) Update(ctx context.Context, id, ownerID, name, kind string) (Widget, error) {
	if name == "" {
		return Widget{}, ErrValidation
	}
	if _, ok, err := uc.store.Get(ctx, id, ownerID); err != nil {
		return Widget{}, err
	} else if !ok {
		return Widget{}, ErrNotFound
	}
	if err := uc.store.Update(ctx, id, ownerID, name, kind); err != nil {
		return Widget{}, err
	}
	w, _, err := uc.store.Get(ctx, id, ownerID)
	if err != nil {
		return Widget{}, err
	}
	uc.publish(ownerID, "widget.updated", w)
	return w, nil
}

// Delete removes a widget and publishes widget.deleted.
func (uc *UseCase) Delete(ctx context.Context, id, ownerID string) error {
	w, ok, err := uc.store.Get(ctx, id, ownerID)
	if err != nil {
		return err
	}
	if !ok {
		return ErrNotFound
	}
	if err := uc.store.Delete(ctx, id, ownerID); err != nil {
		return err
	}
	uc.publish(ownerID, "widget.deleted", w)
	return nil
}

// publish drops the event onto the hub room; delivery is best-effort.
func (uc *UseCase) publish(ownerID, typ string, w Widget) {
	if uc.pub == nil {
		return
	}
	uc.pub.Publish(realtime.Event{
		Room:    realtime.RoomUser(ownerID),
		Type:    typ,
		Payload: widgetPayload(w),
		At:      time.Now(),
	})
}

// widgetPayload is the JSON payload carried on the wire.
func widgetPayload(w Widget) any {
	return struct {
		ID    string `json:"id"`
		Name  string `json:"name"`
		Kind  string `json:"kind"`
		Owner string `json:"owner_user_id"`
	}{w.ID, w.Name, w.Kind, w.OwnerUserID}
}

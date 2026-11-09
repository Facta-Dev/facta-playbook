package widgets

import (
	"encoding/json"
	"fmt"
	"net/http"

	"github.com/google/uuid"

	"github.com/<scope>/<app>/internal/gen/api"
	"github.com/<scope>/<app>/internal/platform/authx"
	"github.com/<scope>/<app>/internal/platform/httpx"
	"github.com/<scope>/<app>/internal/platform/realtime"
)

type handlers struct {
	uc  *UseCase
	pub Publisher
}

// widgetModel maps a slice Widget onto the generated model.
func widgetModel(w Widget) api.Widget {
	return api.Widget{
		Id:          uuidPtr(w.ID),
		Name:        &w.Name,
		Kind:        (*api.WidgetKind)(&w.Kind),
		OwnerUserId: uuidPtr(w.OwnerUserID),
		CreatedAt:   &w.CreatedAt,
	}
}

// readCreate decodes + validates the shared WidgetCreate body (POST and PUT).
func (h handlers) readCreate(w http.ResponseWriter, r *http.Request) (api.WidgetCreate, bool) {
	var req api.WidgetCreate
	if httpx.Decode(w, r, &req) != nil {
		httpx.WriteErr(w, ErrValidation)
		return req, false
	}
	if !req.Kind.Valid() {
		httpx.WriteErr(w, ErrUnknownKind)
		return req, false
	}
	return req, true
}

func (h handlers) list(w http.ResponseWriter, r *http.Request) {
	limit, offset, err := httpx.QueryPage(r)
	if err != nil {
		httpx.WriteErr(w, err)
		return
	}
	rows, err := h.uc.List(r.Context(), ownerID(r), int(limit), int(offset))
	if err != nil {
		httpx.WriteErr(w, err)
		return
	}
	items := make([]api.Widget, 0, len(rows))
	for i := range rows {
		items = append(items, widgetModel(rows[i]))
	}
	httpx.WriteJSON(w, http.StatusOK, api.WidgetList{Items: &items})
}

func (h handlers) create(w http.ResponseWriter, r *http.Request) {
	req, ok := h.readCreate(w, r)
	if !ok {
		return
	}
	row, err := h.uc.Create(r.Context(), ownerID(r), req.Name, string(req.Kind))
	if err != nil {
		httpx.WriteErr(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusCreated, widgetModel(row))
}

func (h handlers) get(w http.ResponseWriter, r *http.Request) {
	row, err := h.uc.Get(r.Context(), r.PathValue("id"), ownerID(r))
	if err != nil {
		httpx.WriteErr(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusOK, widgetModel(row))
}

func (h handlers) update(w http.ResponseWriter, r *http.Request) {
	req, ok := h.readCreate(w, r)
	if !ok {
		return
	}
	row, err := h.uc.Update(r.Context(), r.PathValue("id"), ownerID(r), req.Name, string(req.Kind))
	if err != nil {
		httpx.WriteErr(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusOK, widgetModel(row))
}

func (h handlers) delete(w http.ResponseWriter, r *http.Request) {
	if err := h.uc.Delete(r.Context(), r.PathValue("id"), ownerID(r)); err != nil {
		httpx.WriteErr(w, err)
		return
	}
	w.WriteHeader(http.StatusNoContent)
}

// events streams widget lifecycle events for the caller (SSE over Flusher).
func (h handlers) events(w http.ResponseWriter, r *http.Request) {
	flusher, ok := w.(http.Flusher)
	if !ok {
		httpx.WriteErr(w, ErrValidation)
		return
	}
	ev, cancel := h.pub.Subscribe(realtime.RoomUser(ownerID(r)))
	defer cancel()

	w.Header().Set("Content-Type", "text/event-stream")
	w.Header().Set("Cache-Control", "no-cache")
	w.Header().Set("Connection", "keep-alive")
	w.WriteHeader(http.StatusOK)
	// SSE writes are best-effort: a client disconnect is the normal exit.
	_, _ = fmt.Fprint(w, "retry: 2000\n\n")
	flusher.Flush()

	for {
		select {
		case <-r.Context().Done():
			return
		case e, open := <-ev:
			if !open {
				return
			}
			data, _ := json.Marshal(e.Payload)
			_, _ = fmt.Fprintf(w, "event: %s\ndata: %s\n\n", e.Type, data)
			flusher.Flush()
		}
	}
}

func ownerID(r *http.Request) string {
	id, ok := authx.IdentityFrom(r.Context())
	if !ok {
		return ""
	}
	return id.UserID
}

func uuidPtr(s string) *uuid.UUID {
	u, err := uuid.Parse(s)
	if err != nil {
		return nil
	}
	return &u
}

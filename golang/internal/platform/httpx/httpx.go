// Package httpx holds the HTTP-edge helpers every slice shares: the error
// envelope, JSON decode with validation, and limit/offset pagination.
package httpx

import (
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"strconv"
	"strings"

	"github.com/go-playground/validator/v10"

	"github.com/<scope>/<app>/internal/shared/apperr"
)

// Envelope is the stable error shape (ctx0 parity).
type Envelope struct {
	Detail string `json:"detail"`
	Code   string `json:"code"`
}

var validate = validator.New()

// WriteJSON writes v as JSON with the given status.
func WriteJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(v)
}

// WriteErr writes the {"detail","code"} envelope.
func WriteErr(w http.ResponseWriter, err error) {
	ae := apperr.From(err)
	if ae == nil {
		ae = apperr.E(500, apperr.InternalError, "internal server error")
	}
	WriteJSON(w, ae.HTTP, Envelope{Detail: ae.Detail, Code: ae.Code})
}

// maxBody caps every JSON body at the edge (1 MiB) so no endpoint can
// allocate unbounded memory parsing a request.
const maxBody = 1 << 20

// Decode parses the JSON body (size-capped) and runs validator tags at the
// trust boundary.
func Decode(w http.ResponseWriter, r *http.Request, dst any) error {
	r.Body = http.MaxBytesReader(w, r.Body, maxBody)
	if err := json.NewDecoder(r.Body).Decode(dst); err != nil {
		return apperr.E(400, apperr.Validation, "invalid JSON body")
	}
	if err := validate.Struct(dst); err != nil {
		msg := "invalid request"
		var ves validator.ValidationErrors
		if errors.As(err, &ves) && len(ves) > 0 {
			msg = fmt.Sprintf("%s: failed on '%s'", ves[0].Field(), ves[0].Tag())
		}
		return apperr.E(422, apperr.Validation, msg)
	}
	return nil
}

// RequestIP returns the caller address: the RIGHTMOST hop of
// X-Forwarded-For — the proxy chain appends the real client at the edge
// (nginx $proxy_add_x_forwarded_for), so the last hop is the only
// client-unspoofable one — falling back to RemoteAddr.
func RequestIP(r *http.Request) string {
	if fwd := r.Header.Get("X-Forwarded-For"); fwd != "" {
		parts := strings.Split(fwd, ",")
		return strings.TrimSpace(parts[len(parts)-1])
	}
	return r.RemoteAddr
}

// QueryPage parses limit/offset with sane clamps (limit 1..100, offset >= 0).
func QueryPage(r *http.Request) (limit, offset int32, err error) {
	q := r.URL.Query()
	limit, offset = 20, 0
	if v := q.Get("limit"); v != "" {
		n, perr := strconv.Atoi(v)
		if perr != nil || n < 1 || n > 100 {
			return 0, 0, apperr.E(422, apperr.Validation, "limit must be 1..100")
		}
		limit = int32(n)
	}
	if v := q.Get("offset"); v != "" {
		n, perr := strconv.Atoi(v)
		if perr != nil || n < 0 {
			return 0, 0, apperr.E(422, apperr.Validation, "offset must be >= 0")
		}
		offset = int32(n)
	}
	return limit, offset, nil
}

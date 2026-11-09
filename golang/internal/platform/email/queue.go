// Package email is the "email with job" system: slices enqueue Messages into
// a durable Postgres outbox; an in-process worker claims rows (SELECT ...
// FOR UPDATE SKIP LOCKED), renders the localized template and delivers.
// ctx0 parity: Celery + Redis there — the Postgres outbox is the Go-native,
// zero-broker equivalent.
package email

import (
	"context"
	"encoding/json"
	"log/slog"
	"math/rand/v2"
	"time"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
)

// Message is what a slice wants delivered: template + vars, not rendered bytes.
type Message struct {
	To          string
	TemplateKey string
	Vars        map[string]any
}

// Job is a claimed outbox row inside the worker pipeline.
type Job struct {
	ID          string
	To          string
	TemplateKey string
	Vars        map[string]any
	RetryCount  int
	MaxRetries  int
	ScheduledAt time.Time
}

// Queue is the durable outbox contract (impl: Outbox, pgx adapter).
type Queue interface {
	Enqueue(ctx context.Context, ms Message) error
	Claim(ctx context.Context, now time.Time, limit int) ([]Job, error)
	Complete(ctx context.Context, jobID string) error
	// Fail records the failure; the adapter retries with 2^n-minute backoff
	// and dead-letters (status 'failed') once retry_count >= max_retries.
	Fail(ctx context.Context, job Job, cause error) error
}

// RunWorker drains the outbox until the context is cancelled: claim a batch
// (DB fetch size), render, deliver, then sleep out the poll interval.
func RunWorker(ctx context.Context, q Queue, s Sender, poll time.Duration, batch int) {
	t := time.NewTicker(poll)
	defer t.Stop()
	if batch <= 0 {
		batch = 16
	}
	for {
		drain(ctx, q, s, batch)
		select {
		case <-ctx.Done():
			return
		case <-t.C:
		}
	}
}

func drain(ctx context.Context, q Queue, s Sender, batch int) {
	jobs, err := q.Claim(ctx, time.Now(), batch)
	if err != nil {
		if ctx.Err() != nil {
			return
		}
		slog.Error("email.claim", "err", err)
		return
	}
	for _, j := range jobs {
		lang, _ := j.Vars["locale"].(string)
		subject, body, err := Render(j.TemplateKey, j.Vars, lang)
		if err != nil {
			if ferr := q.Fail(ctx, j, err); ferr != nil {
				slog.Error("email.fail", "job_id", j.ID, "err", ferr)
			}
			continue
		}
		if err := s.Send(ctx, j.To, subject, body); err != nil {
			if ferr := q.Fail(ctx, j, err); ferr != nil {
				slog.Error("email.fail", "job_id", j.ID, "err", ferr)
			}
			continue
		}
		if err := q.Complete(ctx, j.ID); err != nil {
			slog.Error("email.complete", "err", err, "job_id", j.ID)
		}
	}
}

// Outbox is the pgx adapter over the email_jobs table (sqlc-free: the claim
// CTE is the one bespoke query in the template).
type Outbox struct {
	pool *pgxpool.Pool
}

// NewOutbox builds the adapter.
func NewOutbox(pool *pgxpool.Pool) *Outbox { return &Outbox{pool: pool} }

var _ Queue = (*Outbox)(nil)

func (o *Outbox) Enqueue(ctx context.Context, ms Message) error {
	payload, err := json.Marshal(ms.Vars)
	if err != nil {
		return err
	}
	if string(payload) == "null" {
		payload = []byte("{}")
	}
	_, err = o.pool.Exec(ctx,
		`INSERT INTO email_jobs (id, to_email, template_key, payload) VALUES ($1, $2, $3, $4)`,
		uuid.NewString(), ms.To, ms.TemplateKey, payload)
	return err
}

// Claim locks up to `limit` due rows and flips them to 'sending'. The
// second arm recovers rows stranded in 'sending' by a crashed worker
// (process died mid-batch, or shutdown cancelled in-flight sends): they
// become claimable again after the 5-minute lease, at-least-once.
func (o *Outbox) Claim(ctx context.Context, now time.Time, limit int) ([]Job, error) {
	rows, err := o.pool.Query(ctx, `
		WITH picked AS (
			SELECT id FROM email_jobs
			WHERE (status = 'queued' AND scheduled_at <= $1)
			   OR (status = 'sending' AND scheduled_at <= $1 - interval '5 minutes')
			ORDER BY scheduled_at
			LIMIT $2
			FOR UPDATE SKIP LOCKED
		)
		UPDATE email_jobs j SET status = 'sending'
		WHERE j.id IN (SELECT id FROM picked)
		RETURNING j.id, j.to_email, j.template_key, j.payload, j.retry_count, j.max_retries, j.scheduled_at`,
		now, limit)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var jobs []Job
	for rows.Next() {
		var j Job
		var payload []byte
		if err := rows.Scan(&j.ID, &j.To, &j.TemplateKey, &payload, &j.RetryCount, &j.MaxRetries, &j.ScheduledAt); err != nil {
			return nil, err
		}
		if len(payload) > 0 {
			_ = json.Unmarshal(payload, &j.Vars) // malformed payload => nil vars; templates tolerate it
		}
		jobs = append(jobs, j)
	}
	return jobs, rows.Err()
}

func (o *Outbox) Complete(ctx context.Context, jobID string) error {
	_, err := o.pool.Exec(ctx,
		`UPDATE email_jobs SET status = 'sent', sent_at = now(), last_error = NULL WHERE id = $1`,
		jobID)
	return err
}

// Fail bumps retry_count with 2^n-minute jittered backoff; at max_retries the
// job dead-letters (status 'failed') instead of retrying forever.
func (o *Outbox) Fail(ctx context.Context, j Job, cause error) error {
	n := j.RetryCount + 1
	backoff := time.Duration(1<<n) * time.Minute
	// Jitter ±20% OF THE BACKOFF (not the wall clock — with epoch-scale
	// nanos that would reschedule jobs decades away) so simultaneous
	// failures don't re-synchronize polling.
	jitter := time.Duration(rand.Int64N(int64(backoff/5))) - backoff/10
	scheduled := time.Now().Add(backoff).Add(jitter)
	status := "queued"
	if n >= j.MaxRetries {
		status = "failed"
	}
	_, err := o.pool.Exec(ctx, `
		UPDATE email_jobs
		SET status = $2, retry_count = $3, last_error = $4, scheduled_at = $5
		WHERE id = $1`,
		j.ID, status, n, cause.Error(), scheduled)
	return err
}

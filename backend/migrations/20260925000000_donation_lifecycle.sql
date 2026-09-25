-- The original request model has only pending and approved states. There is no
-- cancellation/rejection history, so one active request per donation is valid.
-- Preserve legacy duplicates caused by the old endpoint before enforcing it.
CREATE TABLE donation_request_duplicates_archive (
    id UUID NOT NULL,
    donation_id UUID NOT NULL,
    ngo_id UUID NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    archived_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX donation_request_duplicates_archive_donation_id_idx
    ON donation_request_duplicates_archive (donation_id);

WITH ranked AS (
    SELECT r.id,
           row_number() OVER (
               PARTITION BY r.donation_id
               ORDER BY
                   CASE WHEN r.status = 'aprobada' THEN 0 ELSE 1 END,
                   CASE WHEN d.assigned_ngo_id = r.ngo_id THEN 0 ELSE 1 END,
                   r.created_at,
                   r.id
           ) AS position
    FROM donation_requests r
    JOIN donations d ON d.id = r.donation_id
)
INSERT INTO donation_request_duplicates_archive (id, donation_id, ngo_id, status, created_at)
SELECT r.id, r.donation_id, r.ngo_id, r.status, r.created_at
FROM donation_requests r
JOIN ranked ON ranked.id = r.id
WHERE ranked.position > 1;

DELETE FROM donation_requests r
USING donation_request_duplicates_archive archived
WHERE r.id = archived.id;

CREATE UNIQUE INDEX donation_requests_one_per_donation_idx
    ON donation_requests (donation_id);

UPDATE donations SET status = 'en_acopio' WHERE status IS NULL;

-- Keep existing reservations, but remove legacy assignments that were made
-- before donor approval. Approved requests remain assigned.
UPDATE donations d
SET status = 'reservado'
WHERE d.status = 'en_acopio'
  AND EXISTS (SELECT 1 FROM donation_requests r WHERE r.donation_id = d.id);

UPDATE donations d
SET status = 'en_acopio', assigned_ngo_id = NULL
WHERE d.status = 'reservado'
  AND NOT EXISTS (SELECT 1 FROM donation_requests r WHERE r.donation_id = d.id);

UPDATE donations d
SET assigned_ngo_id = r.ngo_id
FROM donation_requests r
WHERE r.donation_id = d.id
  AND r.status = 'aprobada'
  AND d.status IN ('reservado', 'en_transito', 'entregado', 'rechazado');

UPDATE donations d
SET assigned_ngo_id = NULL
FROM donation_requests r
WHERE r.donation_id = d.id
  AND r.status = 'pendiente'
  AND d.status = 'reservado';

UPDATE donations SET assigned_ngo_id = NULL WHERE status = 'en_acopio';

-- Existing terminal records may predate completed_at and rejection reasons.
-- created_at is the best available timestamp; the placeholder identifies
-- historical missing data rather than presenting it as a real reason.
UPDATE donations
SET completed_at = COALESCE(completed_at, created_at)
WHERE status IN ('entregado', 'rechazado');

UPDATE donations
SET completed_at = NULL
WHERE status IN ('en_acopio', 'reservado', 'en_transito');

UPDATE donations
SET rejection_reason = 'Motivo no registrado en datos históricos'
WHERE status = 'rechazado'
  AND (rejection_reason IS NULL OR btrim(rejection_reason) = '');

UPDATE donations
SET rejection_reason = NULL
WHERE status <> 'rechazado';

ALTER TABLE donations
    ALTER COLUMN status SET DEFAULT 'en_acopio',
    ALTER COLUMN status SET NOT NULL,
    ADD CONSTRAINT donations_completed_at_matches_status CHECK (
        (status IN ('entregado', 'rechazado')) = (completed_at IS NOT NULL)
    ),
    ADD CONSTRAINT donations_rejection_reason_matches_status CHECK (
        (status = 'rechazado' AND rejection_reason IS NOT NULL AND btrim(rejection_reason) <> '')
        OR (status <> 'rechazado' AND rejection_reason IS NULL)
    );

CREATE FUNCTION prevent_delivery_log_mutation() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'delivery_logs is append-only' USING ERRCODE = '55000';
END;
$$;

CREATE TRIGGER delivery_logs_append_only
BEFORE UPDATE OR DELETE ON delivery_logs
FOR EACH ROW EXECUTE FUNCTION prevent_delivery_log_mutation();

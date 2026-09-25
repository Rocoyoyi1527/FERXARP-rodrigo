CREATE TYPE user_role AS ENUM ('admin', 'empresa', 'ong', 'ceo');

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role user_role NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE ngos (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL UNIQUE REFERENCES users(id),
    name TEXT NOT NULL,
    needs_description TEXT,
    latitude DOUBLE PRECISION,
    longitude DOUBLE PRECISION,
    is_verified BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE donations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    title TEXT NOT NULL,
    description TEXT,
    quantity INTEGER NOT NULL,
    status TEXT DEFAULT 'en_acopio' CHECK (
        status IN ('en_acopio', 'reservado', 'en_transito', 'entregado', 'rechazado')
    ),
    assigned_ngo_id UUID REFERENCES ngos(id),
    rejection_reason TEXT,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX donations_user_created_at_idx ON donations (user_id, created_at DESC);
CREATE INDEX donations_status_created_at_idx ON donations (status, created_at DESC);
CREATE INDEX donations_assigned_ngo_id_idx ON donations (assigned_ngo_id);

CREATE TABLE donation_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    donation_id UUID NOT NULL REFERENCES donations(id),
    ngo_id UUID NOT NULL REFERENCES ngos(id),
    status TEXT NOT NULL DEFAULT 'pendiente' CHECK (status IN ('pendiente', 'aprobada')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (donation_id, ngo_id)
);

CREATE INDEX donation_requests_ngo_created_at_idx ON donation_requests (ngo_id, created_at DESC);

CREATE TABLE delivery_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    donation_id UUID NOT NULL REFERENCES donations(id),
    action TEXT NOT NULL CHECK (action IN ('entrada', 'salida', 'entrega', 'rechazo')),
    previous_status TEXT,
    new_status TEXT NOT NULL CHECK (
        new_status IN ('en_acopio', 'reservado', 'en_transito', 'entregado', 'rechazado')
    ),
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX delivery_logs_donation_created_at_idx ON delivery_logs (donation_id, created_at DESC);

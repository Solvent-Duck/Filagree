-- Operational tables for the client portal (Postgres 16).

CREATE TABLE clients (
    client_id     uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    name          text NOT NULL,
    email         text NOT NULL UNIQUE,
    phone         text,
    segment       text NOT NULL CHECK (segment IN ('smb', 'mid', 'enterprise')),
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE contracts (
    contract_id   uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    client_id     uuid NOT NULL REFERENCES clients (client_id),
    plan          text NOT NULL,
    starts_on     date NOT NULL,
    ends_on       date,
    arr_cents     bigint NOT NULL,
    updated_at    timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE invoices (
    invoice_id    uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id   uuid NOT NULL REFERENCES contracts (contract_id),
    amount_cents  bigint NOT NULL,
    status        text NOT NULL CHECK (status IN ('open', 'paid', 'void')),
    issued_on     date NOT NULL,
    updated_at    timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS proposals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    network TEXT NOT NULL,
    account TEXT NOT NULL,
    xdr TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT 'medium',
    expiry INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS signatures (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    proposal_id INTEGER NOT NULL REFERENCES proposals(id),
    signer TEXT NOT NULL,
    signature TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE(proposal_id, signer)
);

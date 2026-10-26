# acme-data

Client data platform for Acme. The client portal and API own the operational
Postgres tables; a nightly extract lands them in S3, Snowpipe loads them into
Snowflake RAW, and dbt builds the LAKE schema analysts query.

| Path | What |
| --- | --- |
| `app/portal` | Client portal (TypeScript) |
| `app/api` | Clients API (Python) |
| `app/db/schema.sql` | Operational Postgres tables |
| `ingest/` | Nightly extract from Postgres to S3 |
| `snowflake/` | Stage, Snowpipe and masking policies |
| `dbt/` | Staging and lake models |

-- External stage over the landing bucket, and one Snowpipe per table into RAW.

CREATE STAGE raw.landing
  URL = 's3://acme-landing/'
  STORAGE_INTEGRATION = acme_landing_s3
  FILE_FORMAT = (TYPE = PARQUET);

CREATE PIPE raw.clients_pipe AUTO_INGEST = TRUE AS
  COPY INTO raw.clients
  FROM @raw.landing/clients/
  MATCH_BY_COLUMN_NAME = CASE_INSENSITIVE;

CREATE PIPE raw.contracts_pipe AUTO_INGEST = TRUE AS
  COPY INTO raw.contracts
  FROM @raw.landing/contracts/
  MATCH_BY_COLUMN_NAME = CASE_INSENSITIVE;

CREATE PIPE raw.invoices_pipe AUTO_INGEST = TRUE AS
  COPY INTO raw.invoices
  FROM @raw.landing/invoices/
  MATCH_BY_COLUMN_NAME = CASE_INSENSITIVE;

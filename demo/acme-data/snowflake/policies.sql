-- PII masking on the lake: only the PII_READER role sees raw email and phone.

CREATE MASKING POLICY lake.mask_pii AS (val string) RETURNS string ->
  CASE WHEN IS_ROLE_IN_SESSION('PII_READER') THEN val ELSE '***MASKED***' END;

ALTER TABLE lake.dim_client MODIFY COLUMN email SET MASKING POLICY lake.mask_pii;
ALTER TABLE lake.dim_client MODIFY COLUMN phone SET MASKING POLICY lake.mask_pii;

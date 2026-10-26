"""Nightly extract: copy changed client tables from Postgres to the S3 landing bucket.

Runs at 02:00 UTC from the scheduler. Each table lands as Parquet under
s3://acme-landing/<table>/dt=<date>/, which Snowpipe picks up.
"""

import datetime as dt
import os

import boto3
import pandas as pd
import psycopg

TABLES = ["clients", "contracts", "invoices"]
BUCKET = "acme-landing"


def extract(since: dt.datetime) -> None:
    s3 = boto3.client("s3")
    today = dt.date.today().isoformat()
    with psycopg.connect(os.environ["APP_DATABASE_URL"]) as conn:
        for table in TABLES:
            df = pd.read_sql(
                f"SELECT * FROM {table} WHERE updated_at >= %(since)s",
                conn,
                params={"since": since},
            )
            key = f"{table}/dt={today}/{table}.parquet"
            path = f"/tmp/{table}.parquet"
            df.to_parquet(path)
            s3.upload_file(path, BUCKET, key)


if __name__ == "__main__":
    extract(dt.datetime.now(dt.UTC) - dt.timedelta(days=1))

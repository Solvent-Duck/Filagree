import os

import asyncpg

pool = asyncpg.create_pool(dsn=os.environ["APP_DATABASE_URL"])

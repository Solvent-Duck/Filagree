"""Clients API: the only writer of the operational client tables."""

from fastapi import APIRouter
from pydantic import BaseModel

from .db import pool

router = APIRouter(prefix="/clients")


class ClientIn(BaseModel):
    name: str
    email: str
    phone: str | None = None
    segment: str


@router.post("")
async def create_client(body: ClientIn):
    async with pool.acquire() as conn:
        row = await conn.fetchrow(
            "INSERT INTO clients (name, email, phone, segment)"
            " VALUES ($1, $2, $3, $4) RETURNING client_id",
            body.name, body.email, body.phone, body.segment,
        )
    return {"client_id": row["client_id"]}


@router.post("/{client_id}/contracts")
async def create_contract(client_id: str, plan: str, starts_on: str, arr_cents: int):
    async with pool.acquire() as conn:
        row = await conn.fetchrow(
            "INSERT INTO contracts (client_id, plan, starts_on, arr_cents)"
            " VALUES ($1, $2, $3, $4) RETURNING contract_id",
            client_id, plan, starts_on, arr_cents,
        )
    return {"contract_id": row["contract_id"]}

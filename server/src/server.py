from fastapi import FastAPI
from fastapi import APIRouter

app = FastAPI()
api_router = APIRouter(prefix="/api/v1")

@api_router.get("/")
async def root():
    return {"message": "Hello World"}


from fastapi import Request, Query
from fastapi.responses import JSONResponse
import uuid
import os
import httpx

SUPABASE_URL = os.environ.get("SUPABASE_URL")
SUPABASE_API_KEY = os.environ.get("SUPABASE_SERVICE_ROLE_KEY")
SUPABASE_TABLE = "rooms"

@api_router.get("/create_room")
async def create_room(
    request: Request,
    formerPort: int = Query(..., ge=0, le=65535),
    formerID: str = Query(None)
):
    # Generate new roomId and formerID if not provided
    room_id = str(uuid.uuid4())
    if formerID is None:
        formerID = str(uuid.uuid4())
    # Get IP address of requester
    client_host = request.client.host

    # Build row for insert
    row = {
        "active": True,
        "roomId": room_id,
        "formerID": formerID,
        "formerIP": client_host,
        "formerPort": formerPort,
        "latterID": None,
        "latterIP": None,
        "latterPort": None
    }

    # Insert into supabase
    url = f"{SUPABASE_URL}/rest/v1/{SUPABASE_TABLE}"
    headers = {
        "apikey": SUPABASE_API_KEY,
        "Authorization": f"Bearer {SUPABASE_API_KEY}",
        "Content-Type": "application/json",
        "Prefer": "return=representation"
    }

    async with httpx.AsyncClient() as client:
        resp = await client.post(url, headers=headers, json=row)
        if resp.status_code not in (200, 201):
            return JSONResponse(
                status_code=resp.status_code,
                content={"error": resp.text}
            )

        # On success, return the created roomId
        return {"roomId": room_id}

app.include_router(api_router)
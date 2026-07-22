from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
import hashlib
import uuid
import os

app = FastAPI()

# in-memory "database": room_id -> room data
rooms: dict[str, dict] = {}


class CreateRoomRequestBody(BaseModel):
    passcode: str
    peer_ip: str
    peer_port: int

class JoinRoomRequestBody(BaseModel):
    passcode: str


class JoinRoomResponseBody(BaseModel):
    peer_ip: str
    peer_port: int


@app.post("/api/v1/room/create")
def create_room(body: CreateRoomRequestBody):
    room_id = str(uuid.uuid4())[:8]  # short id, easy to share/type
    
    passcode_bytes = body.passcode.encode("utf-8")
    passcode_hash = hashlib.sha256(passcode_bytes).hexdigest()
    rooms[room_id] = {
        "passcode": passcode_hash,
        "peer_ip": body.peer_ip,
        "peer_port": body.peer_port,
    }
    return room_id  # matches response.text() on the Rust side


@app.post("/api/v1/room/join/{room_id}")
def join_room(room_id: str, body: JoinRoomRequestBody):
    room = rooms.get(room_id)

    if room is None:
        raise HTTPException(status_code=404, detail="Room not found")

    # The passcode from the client is *not* hashed yet, so we hash it and compare to the stored (hashed) passcode.
    input_passcode_hash = hashlib.sha256(body.passcode.encode("utf-8")).hexdigest()
    if room["passcode"] != input_passcode_hash:
        raise HTTPException(status_code=403, detail="Invalid passcode")

    # Return Peer A's connection info + fingerprint so Peer B can
    # dial them directly and verify their cert against this fingerprint
    return JoinRoomResponseBody(
        peer_ip=room["peer_ip"],
        peer_port=room["peer_port"],
    )

if os.getenv("ENV") != "production":
    @app.get("/api/v1/room/debug")
    def debug_rooms():
        return rooms
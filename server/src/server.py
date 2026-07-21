from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from typing import Optional
import uuid
import os

app = FastAPI()

# in-memory "database": room_id -> room data
rooms: dict[str, dict] = {}


class CreateRoomRequestBody(BaseModel):
    passcode: str
    cert_fingerprint: str
    peer_ip: str
    peer_port: int


class JoinRoomRequestBody(BaseModel):
    passcode: str


class JoinRoomResponseBody(BaseModel):
    cert_fingerprint: str
    peer_ip: str
    peer_port: int


@app.post("/api/v1/room/create")
def create_room(body: CreateRoomRequestBody):
    room_id = str(uuid.uuid4())[:8]  # short id, easy to share/type
    rooms[room_id] = {
        "passcode": body.passcode,
        "cert_fingerprint": body.cert_fingerprint,
        "peer_ip": body.peer_ip,
        "peer_port": body.peer_port,
    }
    return room_id  # matches response.text() on the Rust side


@app.post("/api/v1/room/join/{room_id}")
def join_room(room_id: str, body: JoinRoomRequestBody):
    room = rooms.get(room_id)

    if room is None:
        raise HTTPException(status_code=404, detail="Room not found")

    if room["passcode"] != body.passcode:
        raise HTTPException(status_code=403, detail="Invalid passcode")

    # Return Peer A's connection info + fingerprint so Peer B can
    # dial them directly and verify their cert against this fingerprint
    return JoinRoomResponseBody(
        cert_fingerprint=room["cert_fingerprint"],
        peer_ip=room["peer_ip"],
        peer_port=room["peer_port"],
    )

if os.getenv("ENV") != "production":
    @app.get("/api/v1/room/debug")
    def debug_rooms():
        return rooms
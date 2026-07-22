use redb::{Database, TableDefinition, ReadableTable, ReadableDatabase};
use crate::types::{EngineError, Room};

const ROOMS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("rooms");

pub struct RoomStore {
    db: Database,
}

pub trait RoomOperations {
    fn open(path: &str) -> Result<RoomStore, EngineError>;
    fn store_room(&self, room_id: &str, body: &Room) -> Result<(), EngineError>;
    fn get_room(&self, room_id: &str) -> Result<Option<Room>, EngineError>;
    fn get_rooms(&self) -> Result<Vec<Room>, EngineError>;
    fn delete_room(&self, room_id: &str) -> Result<(), EngineError>;
    fn mark_room_connected(&self, room_id: &str) -> Result<(), EngineError>;
}

impl RoomOperations for RoomStore {
    fn open(path: &str) -> Result<Self, EngineError> {
        log::info!("[db] Opening database at path: {}", path);
        let db = match Database::create(path) {
            Ok(db) => db,
            Err(e) => {
                log::error!("[db] Failed to create database: {}", e);
                return Err(e.into());
            }
        };
        let write_txn = match db.begin_write() {
            Ok(txn) => txn,
            Err(e) => {
                log::error!("[db] Failed to begin write transaction: {}", e);
                return Err(e.into());
            }
        };
        {
            if let Err(e) = write_txn.open_table(ROOMS_TABLE) {
                log::error!("[db] Failed to open 'rooms' table: {}", e);
                return Err(e.into());
            }
        }
        if let Err(e) = write_txn.commit() {
            log::error!("[db] Failed to commit transaction on open: {}", e);
            return Err(e.into());
        }
        log::info!("[db] Database opened successfully at path: {}", path);
        Ok(Self { db })
    }

    fn store_room(&self, room_id: &str, body: &Room) -> Result<(), EngineError> {
        log::info!("[db] Storing room with room_id={}", room_id);
        let serialized = match serde_json::to_vec(body) {
            Ok(data) => data,
            Err(e) => {
                log::error!("[db] Failed to serialize room: {}", e);
                return Err(e.into());
            }
        };
        let write_txn = match self.db.begin_write() {
            Ok(txn) => txn,
            Err(e) => {
                log::error!("[db] Failed to begin write transaction: {}", e);
                return Err(e.into());
            }
        };
        {
            let mut table = match write_txn.open_table(ROOMS_TABLE) {
                Ok(table) => table,
                Err(e) => {
                    log::error!("[db] Failed to open 'rooms' table for store_room: {}", e);
                    return Err(e.into());
                }
            };
            if let Err(e) = table.insert(room_id, serialized.as_slice()) {
                log::error!("[db] Failed to insert room record room_id={}: {}", room_id, e);
                return Err(e.into());
            }
        }
        if let Err(e) = write_txn.commit() {
            log::error!("[db] Failed to commit transaction in store_room: {}", e);
            return Err(e.into());
        }
        log::info!("[db] Room stored successfully, room_id={}", room_id);
        Ok(())
    }

    fn get_room(&self, room_id: &str) -> Result<Option<Room>, EngineError> {
        log::info!("[db] Fetching room by room_id={}", room_id);
        let read_txn = match self.db.begin_read() {
            Ok(txn) => txn,
            Err(e) => {
                log::error!("[db] Failed to begin read transaction: {}", e);
                return Err(e.into());
            }
        };
        let table = match read_txn.open_table(ROOMS_TABLE) {
            Ok(table) => table,
            Err(e) => {
                log::error!("[db] Failed to open 'rooms' table in get_room: {}", e);
                return Err(e.into());
            }
        };

        match table.get(room_id) {
            Ok(opt_value) => {
                match opt_value {
                    Some(value) => {
                        let bytes: &[u8] = value.value();
                        let body: Room = match serde_json::from_slice(bytes) {
                            Ok(body) => {
                                log::debug!("[db] Room deserialized, room_id={}: {:?}", room_id, body);
                                body
                            },
                            Err(e) => {
                                log::error!("[db] Failed to deserialize Room for room_id={}: {}", room_id, e);
                                return Err(e.into());
                            }
                        };
                        Ok(Some(body))
                    }
                    None => {
                        log::debug!("[db] No room found for room_id={}", room_id);
                        Ok(None)
                    }
                }
            }
            Err(e) => {
                log::error!("[db] Failed to fetch room room_id={}: {}", room_id, e);
                Err(e.into())
            }
        }
    }

    fn get_rooms(&self) -> Result<Vec<Room>, EngineError> {
        log::info!("[db] Fetching all rooms");
        let read_txn = match self.db.begin_read() {
            Ok(txn) => txn,
            Err(e) => {
                log::error!("[db] Failed to begin read transaction for get_rooms: {}", e);
                return Err(e.into());
            }
        };
        let table = match read_txn.open_table(ROOMS_TABLE) {
            Ok(table) => table,
            Err(e) => {
                log::error!("[db] Failed to open 'rooms' table in get_rooms: {}", e);
                return Err(e.into());
            }
        };

        let mut rooms = Vec::new();
        let mut room_count = 0u32;

        let iter = match table.iter() {
            Ok(iter) => iter,
            Err(e) => {
                log::error!("[db] Failed to iterate table in get_rooms: {}", e);
                return Err(e.into());
            }
        };

        for entry in iter {
            match entry {
                Ok((_key, value)) => {
                    let bytes: &[u8] = value.value();
                    match serde_json::from_slice(bytes) {
                        Ok(body) => {
                            log::debug!("[db] Got room: {:?}", body);
                            rooms.push(body);
                            room_count += 1;
                        }
                        Err(e) => {
                            log::error!("[db] Failed to deserialize Room in get_rooms: {}", e);
                        }
                    }
                }
                Err(e) => {
                    log::error!("[db] Error iterating rooms table: {}", e);
                }
            }
        }

        log::info!("[db] Total rooms fetched: {}", room_count);
        Ok(rooms)
    }

    fn mark_room_connected(&self, room_id: &str) -> Result<(), EngineError> {
        let mut room = self.get_room(room_id)?
            .ok_or_else(|| EngineError::NotFound(format!("room {room_id} not found")))?;
        room.connected = true;
        self.store_room(room_id, &room)
    }
    fn delete_room(&self, room_id: &str) -> Result<(), EngineError> {
        log::info!("[db] Deleting room, room_id={}", room_id);
        let write_txn = match self.db.begin_write() {
            Ok(txn) => txn,
            Err(e) => {
                log::error!("[db] Failed to begin write transaction for delete_room: {}", e);
                return Err(e.into());
            }
        };
        {
            let mut table = match write_txn.open_table(ROOMS_TABLE) {
                Ok(table) => table,
                Err(e) => {
                    log::error!("[db] Failed to open 'rooms' table for delete_room: {}", e);
                    return Err(e.into());
                }
            };
            if let Err(e) = table.remove(room_id) {
                log::error!("[db] Failed to remove room record, room_id={}: {}", room_id, e);
                return Err(e.into());
            }
        }
        if let Err(e) = write_txn.commit() {
            log::error!("[db] Failed to commit transaction in delete_room: {}", e);
            return Err(e.into());
        }
        log::info!("[db] Room deleted successfully, room_id={}", room_id);
        Ok(())
    }
}
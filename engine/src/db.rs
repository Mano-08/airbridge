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
}

impl RoomOperations for RoomStore {
    fn open(path: &str) -> Result<Self, EngineError> {
        let db = Database::create(path)?;
        let write_txn = db.begin_write()?;
        {
            let _table = write_txn.open_table(ROOMS_TABLE)?;
        }
        write_txn.commit()?;
        Ok(Self { db })
    }

    fn store_room(&self, room_id: &str, body: &Room) -> Result<(), EngineError> {
        let serialized = serde_json::to_vec(body)?;
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(ROOMS_TABLE)?;
            table.insert(room_id, serialized.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    fn get_room(&self, room_id: &str) -> Result<Option<Room>, EngineError> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(ROOMS_TABLE)?;
    
        match table.get(room_id)? {
            Some(value) => {
                let bytes: &[u8] = value.value();
                let body: Room = serde_json::from_slice(bytes)?;
                Ok(Some(body))
            }
            None => Ok(None),
        }
    }

    fn get_rooms(&self) -> Result<Vec<Room>, EngineError> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(ROOMS_TABLE)?;
    
        let mut rooms = Vec::new();
    
        for entry in table.iter()? {
            let (_key, value) = entry?;
            let bytes: &[u8] = value.value();
            let body: Room = serde_json::from_slice(bytes)?;
            rooms.push(body);
        }
    
        Ok(rooms)
    }

    fn delete_room(&self, room_id: &str) -> Result<(), EngineError> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(ROOMS_TABLE)?;
            table.remove(room_id)?;
        }
        write_txn.commit()?;
        Ok(())
    }
}

  
use crate::game::Game;
use color_eyre::eyre::{Result, WrapErr};
use fjall::{Keyspace, PartitionHandle};

pub struct Db {
    pub keyspace: Keyspace,
}

impl Db {
    pub fn open(path: &str) -> Result<Self> {
        let keyspace = fjall::Config::new(path)
            .open()
            .wrap_err_with(|| format!("Failed to open Fjall keyspace at: {path}"))?;
        Ok(Self { keyspace })
    }

    pub fn ign_partition(&self, game: Game) -> Result<PartitionHandle> {
        self.keyspace
            .open_partition(game.partition_key(), Default::default())
            .wrap_err_with(|| format!("Failed to open IGN partition for: {}", game.display()))
    }
}

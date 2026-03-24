pub mod runner;

use crate::db::Db;
use color_eyre::eyre::{Result, WrapErr};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TournamentState {
    Submissions,
    Voting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Match {
    pub a: String,
    pub b: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveMatch {
    pub message_id: u64,
    pub a: String,
    pub b: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tournament {
    pub id: u64,
    pub topic: String,
    pub state: TournamentState,
    pub entries: Vec<String>,
    pub bracket: Vec<Match>,
    pub current_match: Option<ActiveMatch>,
    pub thread_id: u64,
    pub phase_ends_at: u64,
    pub round_secs: u64,
}

const TOURNAMENT_KEY: &[u8] = b"active";

impl Tournament {
    pub fn load(db: &Db) -> Result<Option<Self>> {
        let partition = db.tournament_partition()?;
        let Some(bytes) = partition
            .get(TOURNAMENT_KEY)
            .wrap_err("Failed to read tournament")?
        else {
            return Ok(None);
        };
        let tournament =
            serde_json::from_slice(&bytes).wrap_err("Failed to deserialize tournament")?;
        Ok(Some(tournament))
    }

    pub fn save(&self, db: &Db) -> Result<()> {
        let partition = db.tournament_partition()?;
        let bytes = serde_json::to_vec(self).wrap_err("Failed to serialize tournament")?;
        partition
            .insert(TOURNAMENT_KEY, bytes)
            .wrap_err("Failed to save tournament")?;
        Ok(())
    }

    pub fn delete(db: &Db, _id: u64) -> Result<()> {
        let partition = db.tournament_partition()?;
        partition
            .remove(TOURNAMENT_KEY)
            .wrap_err("Failed to delete tournament")?;
        Ok(())
    }

    pub fn seed_bracket(entries: &[String]) -> Vec<Match> {
        let mut bracket = Vec::new();
        let mut i = 0;
        while i + 1 < entries.len() {
            bracket.push(Match {
                a: entries[i].clone(),
                b: entries[i + 1].clone(),
            });
            i += 2;
        }
        if i < entries.len() {
            bracket.push(Match {
                a: entries[i].clone(),
                b: String::new(),
            });
        }
        bracket
    }
}

use crate::{
    db::Db,
    tournament::{ActiveMatch, Tournament, TournamentState},
};
use color_eyre::eyre::{Result, WrapErr};
use poise::serenity_prelude::{ChannelId, EditThread, Http, MessageId, ReactionType};
use std::sync::Arc;
use tokio::time::{Duration, interval};

pub const REACTION_A: &str = "1️⃣";
pub const REACTION_B: &str = "2️⃣";

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn spawn(http: Arc<Http>, db: Arc<Db>, polling_channel_id: u64) {
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_secs(30));
        loop {
            tick.tick().await;
            if let Err(e) = step(&http, &db, polling_channel_id).await {
                eprintln!("Tournament runner error: {e:?}");
            }
        }
    });
}

pub async fn step(http: &Http, db: &Db, polling_channel_id: u64) -> Result<()> {
    let Some(mut tournament) = Tournament::load(db)? else {
        return Ok(());
    };

    if now_secs() < tournament.phase_ends_at {
        return Ok(());
    }

    match tournament.state {
        TournamentState::Submissions => {
            close_submissions(http, db, &mut tournament, polling_channel_id).await?;
        }
        TournamentState::Voting => {
            advance_voting(http, db, &mut tournament, polling_channel_id).await?;
        }
    }

    Ok(())
}

async fn close_submissions(
    http: &Http,
    db: &Db,
    tournament: &mut Tournament,
    polling_channel_id: u64,
) -> Result<()> {
    let submissions_partition = db.tournament_submissions_partition(tournament.id)?;
    let mut entries: Vec<String> = submissions_partition
        .iter()
        .filter_map(|item| match item {
            Ok((_, val)) => {
                let s = String::from_utf8_lossy(&val).trim().to_string();
                if s.is_empty() { None } else { Some(s) }
            }
            Err(e) => {
                eprintln!("DB error reading submission: {e:?}");
                None
            }
        })
        .collect();

    let thread = ChannelId::new(tournament.thread_id);
    let _ = thread
        .edit_thread(http, EditThread::new().locked(true).archived(true))
        .await;

    let channel = ChannelId::new(polling_channel_id);

    if entries.len() < 2 {
        channel
            .say(
                http,
                format!(
                    "Tournament \"{}\" cancelled — not enough submissions.",
                    tournament.topic
                ),
            )
            .await
            .wrap_err("Failed to post cancellation")?;
        Tournament::delete(db, tournament.id)?;
        return Ok(());
    }

    let seed = tournament.id as usize;
    for i in (1..entries.len()).rev() {
        let j = (seed.wrapping_mul(i + 1)).wrapping_add(i) % (i + 1);
        entries.swap(i, j);
    }

    tournament.entries = entries.clone();
    tournament.bracket = Tournament::seed_bracket(&entries);
    tournament.entries.clear();
    tournament.state = TournamentState::Voting;
    tournament.phase_ends_at = now_secs() + tournament.round_secs;

    tournament.save(db)?;

    channel
        .say(
            http,
            format!(
                "Submissions closed for **{}**. {} entries received. Starting bracket now!",
                tournament.topic,
                entries.len()
            ),
        )
        .await
        .wrap_err("Failed to post submissions closed message")?;

    post_next_match(http, db, tournament, polling_channel_id).await
}

async fn advance_voting(
    http: &Http,
    db: &Db,
    tournament: &mut Tournament,
    polling_channel_id: u64,
) -> Result<()> {
    let channel = ChannelId::new(polling_channel_id);

    if let Some(active) = tournament.current_match.take() {
        match tally_winner(http, polling_channel_id, &active).await? {
            None => {
                eprintln!(
                    "Poll message {} missing, advancing {} by default",
                    active.message_id, active.a
                );
                tournament.entries.push(active.a.clone());
                tournament.save(db)?;
            }
            Some(None) => {
                channel
                    .say(
                        http,
                        format!(
                            "**{}** — it's a tie! Revote!\n{} **{}**\n{} **{}**",
                            tournament.topic, REACTION_A, active.a, REACTION_B, active.b,
                        ),
                    )
                    .await
                    .wrap_err("Failed to post repoll")?;
                tournament.bracket.insert(
                    0,
                    crate::tournament::Match {
                        a: active.a,
                        b: active.b,
                    },
                );
                tournament.phase_ends_at = now_secs() + tournament.round_secs;
                tournament.save(db)?;
                return post_next_match(http, db, tournament, polling_channel_id).await;
            }
            Some(Some(winner)) => {
                tournament.entries.push(winner.clone());
                tournament.save(db)?;

                if tournament.bracket.is_empty() && tournament.entries.len() == 1 {
                    channel
                        .say(
                            http,
                            format!(
                                "The winner of the **{}** tournament is: **{}**!",
                                tournament.topic, winner
                            ),
                        )
                        .await
                        .wrap_err("Failed to post winner")?;
                    Tournament::delete(db, tournament.id)?;
                    return Ok(());
                }

                if tournament.bracket.is_empty() {
                    tournament.bracket = Tournament::seed_bracket(&tournament.entries);
                    tournament.entries.clear();
                }
            }
        }
    }

    tournament.phase_ends_at = now_secs() + tournament.round_secs;
    tournament.save(db)?;

    post_next_match(http, db, tournament, polling_channel_id).await
}

async fn post_next_match(
    http: &Http,
    db: &Db,
    tournament: &mut Tournament,
    polling_channel_id: u64,
) -> Result<()> {
    let channel = ChannelId::new(polling_channel_id);
    loop {
        let Some(next) = tournament.bracket.first() else {
            break;
        };
        match (next.a.is_empty(), next.b.is_empty()) {
            (false, false) => break,
            (false, true) => {
                let winner = tournament.bracket.remove(0).a;
                tournament.entries.push(winner);
            }
            (true, false) => {
                let winner = tournament.bracket.remove(0).b;
                tournament.entries.push(winner);
            }
            (true, true) => {
                tournament.bracket.remove(0);
            }
        }
    }

    if tournament.bracket.is_empty() {
        if tournament.entries.len() <= 1 {
            eprintln!(
                "post_next_match: bracket empty with {} entries, bailing",
                tournament.entries.len()
            );
            return Ok(());
        }
        tournament.bracket = Tournament::seed_bracket(&tournament.entries);
        tournament.entries.clear();
        tournament.save(db)?;
        return Box::pin(post_next_match(http, db, tournament, polling_channel_id)).await;
    }

    let next_match = tournament.bracket.remove(0);

    let msg = channel
        .say(
            http,
            format!(
                "**{}** — vote now!\n{} **{}**\n{} **{}**",
                tournament.topic, REACTION_A, next_match.a, REACTION_B, next_match.b,
            ),
        )
        .await
        .wrap_err("Failed to post matchup")?;

    tournament.current_match = Some(ActiveMatch {
        message_id: msg.id.get(),
        a: next_match.a,
        b: next_match.b,
    });
    tournament.save(db)?;

    let _ = msg
        .react(http, ReactionType::Unicode(REACTION_A.to_string()))
        .await;
    let _ = msg
        .react(http, ReactionType::Unicode(REACTION_B.to_string()))
        .await;

    Ok(())
}

async fn tally_winner(
    http: &Http,
    polling_channel_id: u64,
    active: &ActiveMatch,
) -> Result<Option<Option<String>>> {
    let channel = ChannelId::new(polling_channel_id);

    let message = match channel
        .message(http, MessageId::new(active.message_id))
        .await
    {
        Ok(m) => m,
        Err(_) => return Ok(None),
    };

    let count_for = |emoji: &str| {
        message
            .reactions
            .iter()
            .find(|r| r.reaction_type == ReactionType::Unicode(emoji.to_string()))
            .map(|r| r.count)
            .unwrap_or(0)
    };

    let votes_a = count_for(REACTION_A);
    let votes_b = count_for(REACTION_B);

    if votes_a == votes_b {
        Ok(Some(None))
    } else if votes_a > votes_b {
        Ok(Some(Some(active.a.clone())))
    } else {
        Ok(Some(Some(active.b.clone())))
    }
}

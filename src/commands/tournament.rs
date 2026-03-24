use crate::{
    Context, Error,
    tournament::{Tournament, TournamentState},
};
use color_eyre::eyre::WrapErr;
use poise::serenity_prelude::{ChannelId, ChannelType, CreateThread};

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

async fn is_elder(ctx: Context<'_>) -> Result<bool, Error> {
    let guild_id = match ctx.guild_id() {
        Some(id) => id,
        None => return Ok(false),
    };
    let member = guild_id
        .member(ctx.http(), ctx.author().id)
        .await
        .wrap_err("Failed to fetch guild member")?;
    let elder_role_id = ctx.data().elder_role_id;
    Ok(member.roles.iter().any(|r| r.get() == elder_role_id))
}

#[poise::command(slash_command, rename = "tournament-start")]
pub async fn tournament_start(
    ctx: Context<'_>,
    #[description = "Topic for the tournament"] topic: String,
    #[description = "How many minutes to accept submissions"] submission_mins: u64,
    #[description = "How many minutes each voting round lasts"] round_mins: u64,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    if !is_elder(ctx).await? {
        ctx.send(
            poise::CreateReply::default()
                .content("You need the Elder role to start a tournament.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    if Tournament::load(&ctx.data().db)?.is_some() {
        ctx.send(
            poise::CreateReply::default()
                .content(
                    "A tournament is already running. Cancel it first with /tournament-cancel.",
                )
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    let polling_channel = ChannelId::new(ctx.data().polling_channel_id);
    let now = now_secs();

    let announcement = polling_channel
        .say(
            ctx.http(),
            format!(
                "**{}** tournament started! Post your entry in the thread below. You have {} minutes. One entry per person — your last message is used.",
                topic, submission_mins
            ),
        )
        .await
        .wrap_err("Failed to post tournament announcement")?;

    let thread = polling_channel
        .create_thread_from_message(
            ctx.http(),
            announcement.id,
            CreateThread::new(format!("{} — submissions", topic)).kind(ChannelType::PublicThread),
        )
        .await
        .wrap_err("Failed to create submission thread")?;

    let tournament = Tournament {
        id: now,
        topic: topic.clone(),
        state: TournamentState::Submissions,
        entries: vec![],
        bracket: vec![],
        current_match: None,
        thread_id: thread.id.get(),
        phase_ends_at: now + submission_mins * 60,
        round_secs: round_mins * 60,
    };

    tournament.save(&ctx.data().db)?;

    let _ = ctx
        .send(
            poise::CreateReply::default()
                .content(format!(
                    "Tournament \"{}\" started. Submissions open for {} minutes.",
                    topic, submission_mins
                ))
                .ephemeral(true),
        )
        .await;

    Ok(())
}

#[poise::command(slash_command, rename = "tournament-status")]
pub async fn tournament_status(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    let Some(tournament) = Tournament::load(&ctx.data().db)? else {
        ctx.send(
            poise::CreateReply::default()
                .content("No tournament is currently running.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    let phase = match tournament.state {
        TournamentState::Submissions => {
            let partition = ctx
                .data()
                .db
                .tournament_submissions_partition(tournament.id)?;
            let count = partition.iter().filter(|item| item.is_ok()).count();
            format!("Accepting submissions ({} so far)", count)
        }
        TournamentState::Voting => {
            let remaining = tournament.bracket.len() + tournament.current_match.is_some() as usize;
            let current = tournament
                .current_match
                .as_ref()
                .map(|m| format!("Current match: **{}** vs **{}**", m.a, m.b))
                .unwrap_or_default();
            format!("Voting — {} match(es) remaining\n{}", remaining, current)
        }
    };

    let secs_left = tournament.phase_ends_at.saturating_sub(now_secs());
    let mins_left = secs_left / 60;

    ctx.send(
        poise::CreateReply::default()
            .content(format!(
                "**{}** — {}\nPhase ends in {} minute(s).",
                tournament.topic, phase, mins_left
            ))
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

#[poise::command(slash_command, rename = "tournament-cancel")]
pub async fn tournament_cancel(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    if !is_elder(ctx).await? {
        ctx.send(
            poise::CreateReply::default()
                .content("You need the Elder role to cancel a tournament.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    let Some(tournament) = Tournament::load(&ctx.data().db)? else {
        ctx.send(
            poise::CreateReply::default()
                .content("No tournament is currently running.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    Tournament::delete(&ctx.data().db, tournament.id)?;

    ctx.send(
        poise::CreateReply::default()
            .content(format!("Tournament \"{}\" cancelled.", tournament.topic))
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

#[poise::command(slash_command, rename = "tournament-tick")]
pub async fn tournament_tick(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    if !is_elder(ctx).await? {
        ctx.send(
            poise::CreateReply::default()
                .content("You need the Elder role to use this command.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    ctx.defer_ephemeral().await?;

    crate::tournament::runner::step(ctx.http(), &ctx.data().db, ctx.data().polling_channel_id)
        .await?;

    ctx.send(
        poise::CreateReply::default()
            .content("Tick done.")
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

use crate::{Context, Error, game::Game};
use color_eyre::eyre::WrapErr;
use poise::serenity_prelude::User;

async fn is_elder(ctx: Context<'_>) -> Result<bool, Error> {
    let guild_id = match ctx.guild_id() {
        Some(id) => id,
        None => return Ok(false),
    };
    let member = guild_id
        .member(ctx.http(), ctx.author().id)
        .await
        .wrap_err("Failed to fetch guild member")?;
    Ok(member
        .roles
        .iter()
        .any(|r| r.get() == ctx.data().elder_role_id))
}

#[poise::command(slash_command)]
pub async fn setign(
    ctx: Context<'_>,
    #[description = "Game"] game: Game,
    #[description = "Your in-game name"] ign: String,
) -> Result<(), Error> {
    let user_id = ctx.author().id.to_string();
    let partition = ctx.data().db.ign_partition(game)?;
    partition
        .insert(&user_id, ign.as_bytes())
        .wrap_err("Failed to save IGN")?;

    ctx.send(
        poise::CreateReply::default()
            .content(format!("{} IGN set to {ign}.", game.display()))
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn myigns(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.to_string();
    let db = &ctx.data().db;
    let mut lines = vec!["Your IGNs:".to_string()];

    for game in [Game::NexusStation, Game::PixelWorlds] {
        let partition = db.ign_partition(game)?;
        if let Some(val) = partition.get(&user_id).wrap_err("Failed to read IGN")? {
            let ign = String::from_utf8_lossy(&val);
            lines.push(format!("{}: {ign}", game.display()));
        }
    }

    if lines.len() == 1 {
        ctx.send(
            poise::CreateReply::default()
                .content("You have no IGNs set. Use /setign to add one.")
                .ephemeral(true),
        )
        .await?;
    } else {
        ctx.send(
            poise::CreateReply::default()
                .content(lines.join("\n"))
                .ephemeral(true),
        )
        .await?;
    }

    Ok(())
}

#[poise::command(slash_command)]
pub async fn removeign(ctx: Context<'_>, #[description = "Game"] game: Game) -> Result<(), Error> {
    let user_id = ctx.author().id.to_string();
    let partition = ctx.data().db.ign_partition(game)?;

    match partition.get(&user_id).wrap_err("Failed to read IGN")? {
        Some(_) => {
            partition
                .remove(&user_id)
                .wrap_err("Failed to remove IGN")?;
            ctx.send(
                poise::CreateReply::default()
                    .content(format!("Removed your {} IGN.", game.display()))
                    .ephemeral(true),
            )
            .await?;
        }
        None => {
            ctx.send(
                poise::CreateReply::default()
                    .content(format!("No {} IGN found for your account.", game.display()))
                    .ephemeral(true),
            )
            .await?;
        }
    }

    Ok(())
}

#[poise::command(slash_command)]
pub async fn whois(
    ctx: Context<'_>,
    #[description = "Member to look up"] user: User,
) -> Result<(), Error> {
    let user_id = user.id.to_string();
    let db = &ctx.data().db;
    let mut lines = vec![format!("{}'s IGNs:", user.name)];

    for game in [Game::NexusStation, Game::PixelWorlds] {
        let partition = db.ign_partition(game)?;
        if let Some(val) = partition.get(&user_id).wrap_err("Failed to read IGN")? {
            let ign = String::from_utf8_lossy(&val);
            lines.push(format!("{}: {ign}", game.display()));
        }
    }

    if lines.len() == 1 {
        ctx.send(
            poise::CreateReply::default()
                .content(format!("{} has no IGNs set.", user.name))
                .ephemeral(true),
        )
        .await?;
    } else {
        ctx.send(
            poise::CreateReply::default()
                .content(lines.join("\n"))
                .ephemeral(true),
        )
        .await?;
    }

    Ok(())
}

#[poise::command(slash_command, rename = "eldersetign")]
pub async fn eldersetign(
    ctx: Context<'_>,
    #[description = "Member to update"] user: User,
    #[description = "Game"] game: Game,
    #[description = "Their in-game name"] ign: String,
) -> Result<(), Error> {
    if !is_elder(ctx).await? {
        ctx.send(
            poise::CreateReply::default()
                .content("You need the Elder role to use this command.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    let user_id = user.id.to_string();
    let partition = ctx.data().db.ign_partition(game)?;
    partition
        .insert(&user_id, ign.as_bytes())
        .wrap_err("Failed to save IGN")?;

    ctx.send(
        poise::CreateReply::default()
            .content(format!(
                "Set {}'s {} IGN to {ign}.",
                user.name,
                game.display()
            ))
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

#[poise::command(slash_command, rename = "elderremoveign")]
pub async fn elderremoveign(
    ctx: Context<'_>,
    #[description = "Member to update"] user: User,
    #[description = "Game"] game: Game,
) -> Result<(), Error> {
    if !is_elder(ctx).await? {
        ctx.send(
            poise::CreateReply::default()
                .content("You need the Elder role to use this command.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    let user_id = user.id.to_string();
    let partition = ctx.data().db.ign_partition(game)?;

    match partition.get(&user_id).wrap_err("Failed to read IGN")? {
        Some(_) => {
            partition
                .remove(&user_id)
                .wrap_err("Failed to remove IGN")?;
            ctx.send(
                poise::CreateReply::default()
                    .content(format!("Removed {}'s {} IGN.", user.name, game.display()))
                    .ephemeral(true),
            )
            .await?;
        }
        None => {
            ctx.send(
                poise::CreateReply::default()
                    .content(format!("{} has no {} IGN set.", user.name, game.display()))
                    .ephemeral(true),
            )
            .await?;
        }
    }

    Ok(())
}

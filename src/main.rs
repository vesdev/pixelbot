mod commands;
mod config;
mod db;
mod game;

use argh::FromArgs;
use color_eyre::eyre::WrapErr;
use poise::serenity_prelude as serenity;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(FromArgs)]
/// Pixelbot — clan IGN manager
struct Args {
    /// path to the config file (default: config.toml)
    #[argh(option, short = 'c', default = "PathBuf::from(\"config.toml\")")]
    config: PathBuf,

    /// override the db_path from the config file
    #[argh(option)]
    db_path: Option<String>,
}

pub struct BotData {
    pub db: Arc<db::Db>,
    pub member_role_id: u64,
    pub elder_role_id: u64,
}

pub type Error = color_eyre::eyre::Report;
pub type Context<'a> = poise::Context<'a, BotData, Error>;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let args: Args = argh::from_env();
    let cfg = config::Config::load(&args.config)?;
    let db_path = args
        .db_path
        .as_deref()
        .or(cfg.db_path.as_deref())
        .unwrap_or("./pixelbot.db");

    let data = BotData {
        db: Arc::new(db::Db::open(db_path)?),
        member_role_id: cfg.member_role_id,
        elder_role_id: cfg.elder_role_id,
    };

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::ign::setign(),
                commands::ign::myigns(),
                commands::ign::removeign(),
                commands::ign::whois(),
                commands::ign::eldersetign(),
                commands::ign::elderremoveign(),
            ],
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                match cfg.guild_id {
                    Some(id) => {
                        poise::builtins::register_in_guild(
                            ctx,
                            &framework.options().commands,
                            serenity::GuildId::new(id),
                        )
                        .await
                        .wrap_err("Failed to register commands in guild")?;
                    }
                    None => {
                        poise::builtins::register_globally(ctx, &framework.options().commands)
                            .await
                            .wrap_err("Failed to register commands globally")?;
                    }
                }
                Ok(data)
            })
        })
        .build();

    let intents = serenity::GatewayIntents::non_privileged();

    serenity::ClientBuilder::new(&cfg.token, intents)
        .framework(framework)
        .await
        .wrap_err("Failed to build serenity client")?
        .start()
        .await
        .wrap_err("Client error")?;

    Ok(())
}

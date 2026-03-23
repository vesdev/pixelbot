use poise::ChoiceParameter;

#[derive(Debug, Clone, Copy, ChoiceParameter)]
pub enum Game {
    #[name = "Nexus Station"]
    NexusStation,
    #[name = "Pixel Worlds"]
    PixelWorlds,
}

impl Game {
    pub fn partition_key(self) -> &'static str {
        match self {
            Game::NexusStation => "igns_nexus_station",
            Game::PixelWorlds => "igns_pixel_worlds",
        }
    }

    pub fn display(self) -> &'static str {
        match self {
            Game::NexusStation => "Nexus Station",
            Game::PixelWorlds => "Pixel Worlds",
        }
    }
}

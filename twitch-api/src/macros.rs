macro_rules! twitch_helix {
    ($path:literal) => {
        concat!("https://api.twitch.tv/helix", $path)
    };
}

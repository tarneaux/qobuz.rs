#![allow(clippy::unwrap_used)]

use qobuz::{
    types::{extra::WithExtra, Track},
    Client, QobuzCredentials,
};

#[tokio::main]
async fn main() {
    let client = Client::new(QobuzCredentials::from_env().unwrap())
        .await
        .unwrap();

    let favorites = client
        .get_user_favorites::<Track<WithExtra>>()
        .await
        .unwrap();
    println!("= Favorite tracks =");
    for fav in favorites.iter().map(|v| format!("{v}")) {
        println!("{fav}");
    }

    let playlists = client.get_user_playlists().await.unwrap();
    for playlist in playlists {
        let playlist = client.get_playlist(&playlist.id.to_string()).await.unwrap();
        if playlist.owner.name != "tarneo" {
            continue;
        }
        println!("== {} ==", playlist.name);
        for track in playlist.tracks.items {
            println!("{track}");
        }
    }
}

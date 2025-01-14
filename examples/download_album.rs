// #![allow(clippy::unwrap_used)]

// const DIR: &str = "./music";

// use futures::StreamExt;
// use qobuz::{tag_track, Quality, Track};
// use qobuz::{Client, QobuzCredentials};
// use tokio::fs::File;
// use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() {
    // let client = Client::new(QobuzCredentials::from_env().unwrap())
    //     .await
    //     .unwrap();
    // let album = client.get_album("0794881926329").await.unwrap();

    // let n = album.extra.tracks.items.len();

    // let cover_raw = reqwest::get(album.image.large.clone())
    //     .await
    //     .unwrap()
    //     .bytes()
    //     .await
    //     .unwrap();
    // let cover = audiotags::Picture::new(&cover_raw, audiotags::MimeType::Jpeg);

    // let tracks_and_paths: Vec<(&Track<()>, String)> = album
    //     .extra
    //     .tracks
    //     .items
    //     .iter()
    //     .map(|t| {
    //         let p = format!("{album_dir}/{}.flac", &t.title.replace('/', "-").trim());
    //         (t, p)
    //     })
    //     .collect();

    // for (i, (track, path)) in tracks_and_paths.iter().enumerate() {
    //     let mut bytes_stream = client
    //         .stream_track(&track.id.to_string(), Quality::Cd)
    //         .await
    //         .unwrap();
    //     std::fs::create_dir_all(album_dir.clone()).unwrap();
    //     println!("{}/{}: Downloading {} to {}", i + 1, n, track.title, path);
    //     let mut out = File::create(path.clone()).await.unwrap();
    //     while let Some(item) = bytes_stream.next().await {
    //         tokio::io::copy(&mut item.unwrap().as_ref(), &mut out)
    //             .await
    //             .unwrap();
    //     }
    //     out.flush().await.unwrap();
    //     tag_track(track, path, &album, cover.clone()).unwrap();
    // }
}

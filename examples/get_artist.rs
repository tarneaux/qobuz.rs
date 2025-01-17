#![allow(clippy::unwrap_used)]

use qobuz::{auth::Credentials, Client};

#[tokio::main]
async fn main() {
    let client = Client::new(Credentials::from_env().unwrap()).await.unwrap();
    let artist = client.get_artist("26390").await.unwrap();
    println!("{artist:#?}");
}

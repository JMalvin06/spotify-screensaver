use reqwest::{ self, header::{ ACCEPT, AUTHORIZATION, CONTENT_TYPE } };
use serde::{ Serialize, Deserialize };
use url_search_params;
use core::panic;
use std::collections::HashMap;





// TODO: Handle all errors without panic

#[derive(Serialize, Deserialize, Debug)]
struct AuthResponse {
    access_token: String,
    refresh_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Access {
    access_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Player {
    item: Track,
}

#[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct Track {
    pub name: String,
    pub artists: Vec<Artist>,
    pub album: Album,
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct Album {
    pub images: Vec<Image>,
    pub name: String,
}

impl Default for Album {
    fn default() -> Self {
        Self {
            images: vec![Image::default()],
            name: String::default()
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct Image {
    pub url: String,
}

impl Default for Image {
    fn default() -> Self {
        Self {
            url : "images/placeholder.jpg".to_string()
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct Artist {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Auth {
    refresh: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    display_name: String
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Constants {
    id: String,
    secret: String
}

#[derive(Serialize, Deserialize, Debug)]
struct Playlist {
    context: Context,
}

#[derive(Serialize, Deserialize, Debug)]
struct Context {
    external_urls: URL,
}

#[derive(Serialize, Deserialize, Debug)]
struct URL {
    spotify: String,
}

#[derive(Default, Clone)]
pub struct SpotifyUser {
    token: String,
    id: String,
    secret: String,
    can_recieve: bool,
    current_track: Track,
}

impl SpotifyUser {
    pub fn new() -> SpotifyUser{
        let constants: Constants = serde_json::from_str(include_str!("constants.json")).expect("Could not find constants.json");
        Self{
            id: constants.id,
            secret: constants.secret,
            can_recieve: true,
            ..Default::default()
        }
    }

    pub fn get_image(self) -> Image {
        self.get_track().album.images[0].clone()
    }

    pub fn get_track(self) -> Track {
        self.current_track
    }

    pub fn can_recieve(self) -> bool {
        self.can_recieve
    }

    pub fn get_token(&self) -> &str{
        return &self.token
    }

    pub(crate) async fn generate_token(&mut self) {
        self.can_recieve = false;
        println!("Generating token!");
        // Convert users.json to struct format
        let file: Auth = serde_json
            ::from_str(include_str!("user.json"))
            .expect("Could not convert to json");

        // Format as map to easily access refresh token
        
        let refresh = String::from(file.refresh);

        let auth_url = "https://accounts.spotify.com/api/token";
        let client = reqwest::Client::new();

        // Body parameters for API call
        let params = HashMap::from([
            (String::from("grant_type"), String::from("refresh_token")),
            (String::from("refresh_token"), refresh),
        ]);

        let response = client
            .post(auth_url)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .basic_auth(&self.id, Some(&self.secret)) // Authorize based on client ID and secret
            .body(url_search_params::build_url_search_params(params))
            .send()
            .await 
            .unwrap();

        match response.status() {
            reqwest::StatusCode::OK => {
                match response.json::<Access>().await {
                    Ok(parsed) => {
                        self.token = parsed.access_token;
                        self.can_recieve = true;
                    }
                    Err(e) => panic!("Unexpected error {:?}", e), // Likely invalid authorization code
                }
            }
            other => panic!("there was an unexpected error in the code! {}", other),
        }
    }

    pub(crate) async fn refresh_track(&mut self) {
        self.can_recieve = false;
        let url = format!("https://api.spotify.com/v1/me/player");

        let client = reqwest::Client::new(); // Client to handle API request
        let response = client
            .get(url.clone())
            .header(AUTHORIZATION, format!("Bearer {}",self.token)) // Authorization based on token
            .header(ACCEPT, "application/json") // Recieve json response
            .send()
            .await
            .unwrap();

        match response.status() {
            reqwest::StatusCode::OK => {
                let res = response.text().await.unwrap();
                match serde_json::from_str::<Player>(&res.clone()) {
                    Ok(parsed) => {
                        self.current_track = parsed.item; // Return Track struct
                    }
                    Err(e) => {
                        match serde_json::from_str::<Playlist>(&res.clone()) {
                            Ok(parsed) => {
                                if parsed.context.external_urls.spotify.contains("37i9dQZF1EYkqdzj48dyYq")
                                {
                                    self.current_track =  Track {
                                        name: String::from("Up next"),
                                        artists: vec![Artist { name: String::from("DJ X") }],
                                        album: Album {
                                            images: vec![Image {
                                                url: String::from(
                                                    "https://lexicon-assets.spotifycdn.com/DJ-Beta-CoverArt-640.jpg"
                                                ),
                                            }],
                                            name: String::default(),
                                        },
                                    };
                                } else {
                                    panic!("Response did not match the structure: {:?}", e)
                                }
                            }
                            Err(e) => panic!("Response did not match the structure: {:?}", e),
                        }
                    }
                }
            }
            reqwest::StatusCode::UNAUTHORIZED => {
                println!("Failed to aquire data..\nGenerating new token...");
                self.token = String::new();
                self.can_recieve = true;
                return;
            }
            reqwest::StatusCode::NO_CONTENT => {
                self.current_track = Track::default();
            }
            _other => ()
        }
        self.can_recieve = true;
    }

    pub(crate) async fn get_image_data(&self) -> bytes::Bytes {
        let img_bytes = match reqwest
            ::get(self.clone().get_image().url).await {
                Ok(parsed) => {
                    parsed.bytes().await.expect("could not convert")
                },
                Err(_) => {
                    return bytes::Bytes::new()
                },
            };
            
        return img_bytes;
    }
}
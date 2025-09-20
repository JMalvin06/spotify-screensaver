use reqwest::{ self, header::{ ACCEPT, AUTHORIZATION, CONTENT_TYPE } };
use serde::{ Serialize, Deserialize };
use url_search_params;
use open;
use tokio::{self};

use core::panic;
use std::{io::{Read, Write}, net::TcpListener};
use std::collections::HashMap;
use std::fs;


const URI: &str = "http://localhost:8888/callback";

// TODO: Handle all errors without panic
// TODO: Organize structs

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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct Track {
    pub name: String,
    pub artists: Vec<Artist>,
    pub album: Album,
}

impl Default for Track {
    fn default() -> Self {
        Self { 
            name: "Nothing Playing".to_string(), 
            artists: Vec::default(), 
            album: Album::default()
        }
    }
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

/*#[derive(Serialize, Deserialize, Debug)]
struct Users {
    users: Vec<User>,
}*/

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
    pub(crate) username: String,
    token: String,
    current_track: Track,
    id: String,
    secret: String
}




impl SpotifyUser {

    pub fn new() -> SpotifyUser{
        Self::default()
    }

    pub fn get_track(self) -> Track {
        self.current_track
    }

    pub fn get_image(self) -> Image {
        self.get_track().album.images[0].clone()
    }
    
    pub fn token_empty(self) -> bool {
        self.token.is_empty()
    }

    pub fn set_id(&mut self, id: String) {
        self.id = id;
    }

    pub fn set_secret(&mut self, secret: String) {
        self.secret = secret;
    }

    #[tokio::main]
    pub async fn set_username(&mut self) {
        let url = format!("https://api.spotify.com/v1/me");

        let client = reqwest::Client::new();
        let response = client
            .get(url.clone())
            .bearer_auth(self.token.clone())
            .header(ACCEPT, "application/json") // Recieve json response
            .send()
            .await
            .unwrap();

        match response.status() {
            reqwest::StatusCode::OK => {
                let res = response.text().await.unwrap();
                match serde_json::from_str::<User>(&res.clone()) {
                    Ok(user) => {
                        println!("NAME SET TO: {}", user.display_name);
                        self.username = user.display_name
                    }
                    Err(_) => {
                        panic!("Could not generate username")
                    }
                }
            }
            reqwest::StatusCode::UNAUTHORIZED => {
                panic!("Unauthorized");
            }
            reqwest::StatusCode::NO_CONTENT => {
                panic!("No content")
            }
            _other => {
                panic!("panicked with status code: {}", response.text().await.unwrap())
            }
        }
    }

    // TODO: Organize functions
    

    #[tokio::main]
    pub(crate) async fn generate_token(&mut self) {
        println!("Generating token!");
        // Convert user.json to struct format
        let file: Auth = serde_json
            ::from_str(fs::read_to_string("user.json").expect("Error opening file").as_str())
            .expect("Could not convert to json");

        // Format as map to easily access refresh token
        /*let user_map: HashMap<String, String> = file.users
            .iter()
            .map(|u| (u.name.clone(), u.refresh.clone()))
            .collect();*/
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
            .basic_auth(self.id.clone(), Some(self.secret.clone())) // Authorize based on client ID and secret
            .body(url_search_params::build_url_search_params(params))
            .send().await
            .unwrap();
        match response.status() {
            reqwest::StatusCode::OK => {
                match response.json::<Access>().await {
                    Ok(parsed) => {
                        self.token = parsed.access_token // Recieve access token
                    }
                    Err(e) => {
                        println!("There was an error: {e}");
                        self.token = String::new();
                    }, // Likely invalid authorization code
                }
            }
            other => panic!("there was an unexpected error in the code! {}", other),
        }
    }

    pub(crate) fn retrieve_code(&self) -> String {
        let scope = "user-read-private user-read-email user-read-playback-state";
        // Parameters for body of URL link
        let params = HashMap::from([
            (String::from("response_type"), String::from("code")),
            (String::from("client_id"), String::from(self.id.clone())),
            (String::from("scope"), String::from(scope)),
            (String::from("redirect_uri"), String::from(URI)),
        ]);

        // Open listener at URI address
        let listener = match  TcpListener::bind("127.0.0.1:8888"){
            Ok(l) => l,
            Err(e) => {panic!("Error Code: {:?}", e)},
        };

        let url_out = format!(
            "https://accounts.spotify.com/authorize?{}",
            url_search_params::build_url_search_params(params)
        );
        println!("Opening {} on default browser", url_out);
        open::that(url_out).expect("Not a valid URL"); // Automatically open browser at link

        let mut code: String = String::default();
        while code.is_empty() {
            for stream in listener.incoming() {
                    let mut req = &stream.unwrap().try_clone().unwrap();
                    let mut buffer = [0; 512];
                    req.read(&mut buffer).unwrap();
                    if buffer.starts_with(b"GET /callback?code="){
                        code = String::from_utf8_lossy(&buffer[19..275]).to_string();
                        let status_line = "HTTP/1.1 200 OK";
                        let contents = include_str!("response.html");
                        let length = contents.len();
                        let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

                        // Write and push html response
                        req.write_all(response.as_bytes()).expect("Unable to write");
                        req.flush().expect("Unable to send request");
                        break;
                    }
            }
        }

        println!("Code: {}", code);
        code // Return authorization code
    }

    #[tokio::main]
    pub(crate) async fn refresh_track(&mut self) {
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
                return;
            }
            reqwest::StatusCode::NO_CONTENT => {
                self.current_track = Track::default();
            }
            _other => ()
        }
    }

    #[tokio::main]
    pub(crate) async fn get_image_data(&self) -> bytes::Bytes {
        let img_bytes = reqwest
            ::get(self.clone().get_image().url).await
            .expect("bad response")
            .bytes().await
            .expect("could not convert");
        return img_bytes;
    }

    #[tokio::main]
    pub(crate) async fn generate_user(self) -> String {
        let code = self.retrieve_code();
        println!("Generating Token...");
        let auth_url = "https://accounts.spotify.com/api/token";

        // Parameters for body of API call
        let params = HashMap::from([
            (String::from("grant_type"), String::from("authorization_code")),
            (String::from("code"), String::from(code)),
            (String::from("redirect_uri"), String::from(URI.trim())),
        ]);

        //println!("ID: {id}");
        let client = reqwest::Client::new(); // Initialize client to handle API call
        let response = client
            .post(auth_url)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .basic_auth(self.id.clone(), Some(self.secret.clone()))
            .body(url_search_params::build_url_search_params(params))
            .header(ACCEPT, "application/json") // Recieve response in json format
            .send().await
            .unwrap();

        match response.status() {
            reqwest::StatusCode::OK => {
                match response.json::<AuthResponse>().await {
                    Ok(parsed) => {
                        let refresh_token = parsed.refresh_token;
                        let file: Auth = Auth {
                            refresh: String::from(refresh_token),
                        };
                            
                        fs::write(
                            "user.json",
                            serde_json::to_string_pretty(&file).expect("Could not convert")
                        ).expect("Could not write");
                        let constants_file: Constants = Constants{ 
                            id: self.id, 
                            secret: self.secret
                        };
                        fs::write(
                            "constants.json", 
                        serde_json::to_string_pretty(&constants_file).expect("could not convert")).expect("Failed to write to constants.json");

                        parsed.access_token // Return recieved access token
                    }
                    Err(e) => panic!("the response did not match the struct {:?}", e),
                }
            }
            other => panic!("there was an unexpected error in the code! {:?}", response.text().await.unwrap())
        }
    }

}
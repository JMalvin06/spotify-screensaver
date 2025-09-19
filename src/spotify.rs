use reqwest::{ self, header::{ ACCEPT, AUTHORIZATION, CONTENT_TYPE } };
use serde::{ Serialize, Deserialize };
use url_search_params;
use open;
use tokio::{self};

use std::{fs::File, io::{BufReader, Read, Write}, net::TcpListener};
use std::collections::HashMap;
use std::fs;


mod constants; // Stores client ID and secret for safekeeping
const URI: &str = "http://localhost:8888/callback";
const ID: &str = constants::ID;
const SECRET: &str = constants::SECRET;

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

#[derive(Serialize, Deserialize, Debug)]
struct Users {
    users: Vec<User>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct User {
    name: String,
    refresh: String,
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
    current_track: Track
}




impl SpotifyUser {

    pub fn new() -> SpotifyUser{
        Self { 
            token: String::new(), 
            current_track: Track::default()
        }
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

    // TODO: Organize functions
    

    #[tokio::main]
    pub(crate) async fn generate_token(&mut self, username: String) {
        println!("Generating token!");
        // Convert users.json to struct format
        let file: Users = serde_json
            ::from_str(fs::read_to_string("users.json").expect("Error opening file").as_str())
            .expect("Could not convert to json");

        // Format as map to easily access refresh token
        let user_map: HashMap<String, String> = file.users
            .iter()
            .map(|u| (u.name.clone(), u.refresh.clone()))
            .collect();
        let refresh = String::from(user_map.get(username.trim()).expect("Not found"));

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
            .basic_auth(ID, Some(SECRET)) // Authorize based on client ID and secret
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
        let scope = "user-read-playback-state";
        // Parameters for body of URL link
        let params = HashMap::from([
            (String::from("response_type"), String::from("code")),
            (String::from("client_id"), String::from(ID)),
            (String::from("scope"), String::from(scope)),
            (String::from("redirect_uri"), String::from(URI)),
        ]);

        // Open listener at URI address
        let listener = match  TcpListener::bind("127.0.0.1:8888"){
            Ok(l) => l,
            Err(e) => {panic!("Error Code: {:?}", e)
    },
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
                        code = String::from_utf8_lossy(&buffer[19..227]).to_string();
                        let status_line = "HTTP/1.1 200 OK";
                        let contents = fs::read_to_string("response.html").expect("Unable to read file");
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
            .send().await
            .unwrap();

        match response.status() {
            reqwest::StatusCode::OK => {
                let response = response.text().await.unwrap();
                match serde_json::from_str::<Player>(&response.clone()) {
                    Ok(parsed) => {
                        self.current_track = parsed.item; // Return Track struct
                    }
                    Err(e) => {
                        match serde_json::from_str::<Playlist>(&response.clone()) {
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
            // TODO: Add logic to generate new token
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

    


    
}

// Generates the list of users logged in
    pub(crate) fn get_user_list() -> Vec<String> {
        let file = match fs::read_to_string("users.json"){
            Ok(f) => f,
            Err(_) => return vec![]
        };

        let user_list: Vec<User> = match serde_json::from_str::<Users>(file.as_str()) {
            Ok(f) => f.users,
            Err(_) => return vec![],
        };

        user_list
            .iter()
            .map(|u| u.name.clone())
            .collect() // Parse json file and generate a list of usernames
    }

    pub(crate) fn delete_user(to_delete: String) {
        let file = File::open("users.json").expect("could not open file");
        let reader = BufReader::new(file);
        let mut file: Users = match serde_json::from_reader(reader) {
            Ok(f) => f,
            Err(_) => {
                fs::write("users.json", "{\"users\": []}").expect("Failed to write");
                return;
            }
        };

        for user in file.users.iter().enumerate(){
            if user.1.name.trim() == to_delete.trim() {
                file.users.remove(user.0);
                fs::write("users.json", serde_json::to_string_pretty(&file).expect("Could not convert")).expect("Could not write");
                return;
            }
        }
    }

#[tokio::main]
pub(crate) async fn generate_user(username: String, client: SpotifyUser) -> String {
    let code = client.retrieve_code();
    println!("Generating Token...");
    let auth_url = "https://accounts.spotify.com/api/token";

    // Parameters for body of API call
    let params = HashMap::from([
        (String::from("grant_type"), String::from("authorization_code")),
        (String::from("code"), String::from(code)),
        (String::from("redirect_uri"), String::from(URI.trim())),
    ]);

    let client = reqwest::Client::new(); // Initialize client to handle API call
    let response = client
        .post(auth_url)
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .basic_auth(ID, Some(SECRET))
        .body(url_search_params::build_url_search_params(params))
        .header(ACCEPT, "application/json") // Recieve response in json format
        .send().await
        .unwrap();

    match response.status() {
        reqwest::StatusCode::OK => {
            match response.json::<AuthResponse>().await {
                Ok(parsed) => {
                    let mut file: Users = match fs::read_to_string("users.json") {
                        Ok(f) => {
                            // Check if users.json has been corrupted, and correct if necessary
                            match serde_json::from_str::<Users>(f.as_str()){
                                Ok(f) =>  {
                                    f
                                },
                                Err(_) =>  {
                                    fs::write("users.json", "{\"users\": []}").expect("Failed to write");
                                    Users {users: vec![]}
                                }
                            }
                        }
                        Err(_) => {
                            println!("users.json does not exist, creating..");
                            fs::write("users.json", "{\"users\": []}").expect("Failed to write");
                            Users {users: vec![]}
                        }
                    };
                        

                    let refresh_token = parsed.refresh_token;
                    let mut user_exists = -1;

                    // Check if user exists already to avoid duplicate entries
                    for user in file.users.iter().enumerate() {
                        if user.1.name.trim() == username.trim() {
                            user_exists = user.0 as i32; // Assign to index where user is present
                            break;
                        }
                    }

                    // Assign user with refresh token in users.json
                    if user_exists == -1 {
                        // if user does NOT exist, add new json entry
                        println!(
                            "Old json: {}",
                            serde_json::to_string_pretty(&file).expect("Oopsie")
                        );
                        let new_user: User = User {
                            name: String::from(username.trim()),
                            refresh: String::from(refresh_token),
                        };
                        file.users.push(new_user);
                    } else {
                        // if user DOES exist, edit existing json entry
                        let new_user = file.users.get_mut(user_exists as usize).unwrap();
                        new_user.refresh = String::from(refresh_token);
                    }
                    println!("New json: {}", serde_json::to_string_pretty(&file).expect("Oopsie"));
                    fs::write(
                        "users.json",
                        serde_json::to_string_pretty(&file).expect("Could not convert")
                    ).expect("Could not writes");
                    parsed.access_token // Return recieved access token
                }
                Err(e) => panic!("the response did not match the struct {:?}", e),
            }
        }
        other => panic!("there was an unexpected error in the code! {:?}", other),
    }
}
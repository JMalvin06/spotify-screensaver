use reqwest::{ self, header::{ ACCEPT, AUTHORIZATION, CONTENT_TYPE } };
use serde::{ Serialize, Deserialize };
use url_search_params;
use open;
use tokio;
use std::sync::mpsc;

use std::process::Command;
use std::collections::HashMap;
use std::fs;

use crate::set_can_recieve;

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

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Track {
    pub name: String,
    pub artists: Vec<Artist>,
    pub album: Album,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Album {
    pub images: Vec<Image>,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Image {
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug)]
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

// TODO: Organize functions
#[tokio::main]
pub(crate) async fn generate_refresh(username: String, code: String) -> String {
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
                    // TODO: Handle json errors less repetitively
                    let file = match fs::read_to_string("users.json") {
                        Ok(f) => {
                            // Cleans whitespace from file (trim does not work for some reason)
                            let cleaned_file: String = f
                                .chars()
                                .filter(|c| !c.is_whitespace())
                                .collect();

                            // Check if users.json does not follow necessary format, and correct
                            if
                                f.is_empty() ||
                                (!cleaned_file.contains("{\"users\":[") &&
                                    !cleaned_file.contains("]}"))
                            {
                                fs::write("users.json", "{\"users\": []}").expect(
                                    "Failed to write"
                                );
                                fs::read_to_string("users.json").expect("Terrible horrible error")
                            } else {
                                f
                            }
                        }
                        Err(_) => {
                            println!("users.json does not exist, creating..");
                            fs::write("users.json", "{\"users\": []}").expect("Failed to write");
                            fs::read_to_string("users.json").expect("Terrible horrible error")
                        }
                    };
                    let mut file: Users = serde_json::from_str(file.as_str()).expect("Time to cry");
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


pub(crate) async fn generate_token(sender: mpsc::Sender<String>, user: String) {
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
    let refresh = String::from(user_map.get(user.trim()).expect("Not found"));

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
                    println!("Generated");
                    //set_can_recieve(true);
                    sender.send(parsed.access_token); // Recieve access token
                }
                Err(e) => panic!("Unexpected error {:?}", e), // Likely invalid authorization code
            }
        }
        other => panic!("there was an unexpected error in the code! {}", other),
    }
}

pub(crate) fn retrieve_code() -> String {
    let scope = "user-read-playback-state";
    // Parameters for body of URL link
    let params = HashMap::from([
        (String::from("response_type"), String::from("code")),
        (String::from("client_id"), String::from(ID)),
        (String::from("scope"), String::from(scope)),
        (String::from("redirect_uri"), String::from(URI)),
    ]);

    //File::create("code.txt").expect("Could not create file");
    // Open node.js server to listen for authorization code at URI
    Command::new("node").arg("server.js").spawn().expect("Failed to start");

    let url_out = format!(
        "https://accounts.spotify.com/authorize?{}",
        url_search_params::build_url_search_params(params)
    );
    println!("Opening {} on default browser", url_out);
    open::that(url_out).expect("Not a valid URL"); // Automatically open browser at link

    //let mut response = String::new();
    // TODO: Automatically proceed with code once code has been recieved
    loop {
        if fs::exists("code.txt").expect("File error") {
            if !fs::read("code.txt").expect("Could not open").is_empty() {
                println!("Found code");
                break;
            }
        }
    }
    //while fs::read("code.txt").expect("Could not open").is_empty(){}

    // Close the javascript server
    // TODO: Close specific "server.js" server
    Command::new("killall").arg("node").spawn().expect("Failed to kill");

    let code = fs::read_to_string("code.txt").expect("Could not read file");
    fs::remove_file("code.txt").expect("Failed to delete file"); // Remove file with code for security purposes
    println!("Code: {}", code);
    code // Return authorization code
}


pub(crate) async fn get_current_track(sender: mpsc::Sender<bytes::Bytes>, auth_token: String) {
    let url = format!("https://api.spotify.com/v1/me/player");

    let client = reqwest::Client::new(); // Client to handle API request
    let response = client
        .get(url.clone())
        .header(AUTHORIZATION, format!("Bearer {auth_token}")) // Authorization based on token
        .header(ACCEPT, "application/json") // Recieve json response
        .send().await
        .unwrap();

    match response.status() {
        reqwest::StatusCode::OK => {
            match response.json::<Player>().await {
                Ok(parsed) => {
                    println!("Can recieve");
                    set_can_recieve(true);
                    sender.send(get_image(parsed.item.album.images.get(1).expect("Could not load").url.clone()).await).expect("Could not send");
                }
                Err(e) => {
                    // TODO: Clean redundant API call
                    let response = client
                        .get(url)
                        .header(AUTHORIZATION, format!("Bearer {auth_token}")) // Authorization based on token
                        .header(ACCEPT, "application/json") // Recieve json response
                        .send().await
                        .unwrap();

                    match response.json::<Playlist>().await {
                        Ok(parsed) => {
                            if
                                parsed.context.external_urls.spotify.contains(
                                    "37i9dQZF1EYkqdzj48dyYq"
                                )
                            {
                                /*return Track {
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
                                };*/
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
        reqwest::StatusCode::UNAUTHORIZED => { panic!("new authentication code needed") }
        reqwest::StatusCode::NO_CONTENT => {
            /*let empty = Track {
                name: String::default(),
                artists: vec![],
                album: Album {
                    name: String::default(),
                    images: vec![],
                },
            };
            return empty;*/
        }
        other => panic!("there was an unexpected error: {:?}", other),
    }
}

pub(crate) async fn get_image(url: String) -> bytes::Bytes{
    let response = match reqwest::get(url).await {
            Ok(r) => r,
            Err(_) => return bytes::Bytes::new()
        };

        let img_bytes = match response.bytes().await {
            Ok(b) => b,
            Err(_) => return bytes::Bytes::new()
        };

    img_bytes
}

/// Generates the list of users logged in
pub(crate) fn get_user_list() -> Vec<String> {
    let file: Users = serde_json
        ::from_str(fs::read_to_string("users.json").expect("Error opening file").as_str())
        .expect("Could not convert to json");
    file.users
        .iter()
        .map(|u| u.name.clone())
        .collect() // Parse json file and generate a list of usernames
}

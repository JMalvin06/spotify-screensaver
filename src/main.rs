use reqwest::{self, header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE}};
use serde::{Serialize, Deserialize};
use url_search_params;
use open;

use std::{io::{self, stdin, stdout, Write}, process::Command};
use std::collections::HashMap;
use std::fs;

mod constants; // Stores client ID and secret for safekeeping
const URI: &str = "http://localhost:8888/callback";
const ID: &str = constants::ID;
const SECRET: &str = constants::SECRET;


#[derive(Serialize, Deserialize, Debug)]
struct AuthResponse{
    access_token: String,
    refresh_token: String
}

#[derive(Serialize, Deserialize, Debug)]
struct Access{
    access_token: String
}

#[derive(Serialize, Deserialize, Debug)]
struct Player{
    item: Track
}


#[derive(Serialize, Deserialize, Debug)]
struct Track{
    name: String,
    artists: Vec<Artist>
}

#[derive(Serialize, Deserialize, Debug)]
struct Artist{
    name: String
}


#[derive(Serialize, Deserialize, Debug)]
struct Users {
    users: Vec<User>
}


#[derive(Serialize, Deserialize, Debug, Clone)]
struct User {
    name: String,
    refresh: String
}


async fn generate_refresh(username: &String, code: String) -> String{
    println!("Generating Token...");
    let auth_url = "https://accounts.spotify.com/api/token"; 

    // Parameters for body of API call
    let params = HashMap::from([
        (String::from("grant_type"), String::from("authorization_code")),
        (String::from("code"), String::from(code)),
        (String::from("redirect_uri"), String::from(URI.trim()))
    ]
    );
    

    let client = reqwest::Client::new(); // Initialize client to handle API call
    let response = client
    .post(auth_url)
    .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
    .basic_auth(ID, Some(SECRET))
    .body(url_search_params::build_url_search_params(params)) 
    .header(ACCEPT, "application/json") // Recieve response in json format
    .send()
    .await
    .unwrap();

    match response.status() {
        reqwest::StatusCode::OK => {
            match response.json::<AuthResponse>().await {
                Ok(parsed) => {
                    // TODO: Handle json errors less repetitively
                    let file = match fs::read_to_string("users.json"){ 
                        Ok(f) => {
                            // Check if users.json does not follow necessary format, and correct
                            if f.is_empty() || (!f.trim().contains("{\"users\": [") && !f.trim().contains("]}")){
                                fs::write("users.json", "{\"users\": []}").expect("Failed to write");
                                fs::read_to_string("users.json").expect("Terrible horrible error")
                            } else{
                                f
                            }
                        },
                        Err(_) => {
                            println!("users.json does not exist, creating..");
                            fs::write("users.json", "{\"users\": []}").expect("Failed to write");
                            fs::read_to_string("users.json").expect("Terrible horrible error")
                        }
                    };
                    let mut file: Users = serde_json::from_str(file.as_str()).expect("I will kill you");
                    let refresh_token = parsed.refresh_token;
                    let mut user_exists = -1;

                    // Check if user exists already to avoid duplicate entries
                    for user in file.users.iter().enumerate(){
                        if user.1.name.trim() == username.trim(){
                            user_exists = user.0 as i32; // Assign to index where user is present
                            break;
                        }
                    }
                    
                    // Assign user with refresh token in users.json
                    if user_exists == -1 { // if user does NOT exist, add new json entry
                        let new_user: User = User{
                            name: String::from(username.trim()),
                            refresh: String::from(refresh_token)
                        };
                        file.users.push(new_user);
                    } else { // if user DOES exist, edit existing json entry
                        let new_user = file.users.get_mut(user_exists as usize).unwrap();
                        new_user.refresh = String::from(refresh_token);
                    }
                    fs::write("users.json", serde_json::to_string_pretty(&file).expect("Could not convert")).expect("Could not writes");
                    parsed.access_token // Return recieved access token
                }
                Err(e) => panic!("the response did not match the struct {:?}", e)
            }
        }
        other => panic!("there was an unexpected error in the code! {:?}", other)
    }
}

async fn generate_token(user: &String) -> String{
    // Convert users.json to struct format
    let file: Users = serde_json::from_str(
        fs::read_to_string("users.json")
        .expect("Error opening file")
        .as_str()
    ).expect("Could not convert to json");

    // Format as map to easily access refresh token
    let user_map: HashMap<String, String> = file.users.iter().map(|u| (u.name.clone(), u.refresh.clone())).collect(); 
    let refresh = String::from(user_map.get(user.trim()).expect("Not found"));

    let auth_url = "https://accounts.spotify.com/api/token";
    let client = reqwest::Client::new();

    // Body parameters for API call
    let params = HashMap::from([
        (String::from("grant_type"), String::from("refresh_token")),
        (String::from("refresh_token"), refresh)
    ]
    );

    let response = client
    .post(auth_url)
    .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
    .basic_auth(ID, Some(SECRET)) // Authorize based on client ID and secret
    .body(url_search_params::build_url_search_params(params))
    .send()
    .await
    .unwrap();

    match response.status() {
        reqwest::StatusCode::OK => {
            match response.json::<Access>().await {
                Ok(parsed) => {
                    parsed.access_token // Recieve access token
                },
                Err(e) => panic!("Unexpected error {:?}", e) // Likely invalid authorization code
            }
        }
        other => panic!("there was an unexpected error in the code! {}", other)

    }
}

fn retrieve_code() -> String {
    let scope = "user-read-playback-state";
    // Parameters for body of URL link
    let params = HashMap::from([
        (String::from("response_type"), String::from("code")),
        (String::from("client_id"), String::from(ID)),
        (String::from("scope"), String::from(scope)),
        (String::from("redirect_uri"), String::from(URI)),
    ]
    );

    // Open node.js server to listen for authorization code at URI
    Command::new("node")
    .arg("server.js")
    .spawn()
    .expect("Failed to start");

    let url_out = format!("https://accounts.spotify.com/authorize?{}", url_search_params::build_url_search_params(params));
    println!("Opening {} on default browser", url_out);
    open::that(url_out).expect("Not a valid URL"); // Automatically open browser at link

    let mut response = String::new();
    // TODO: Automatically proceed with code once code has been recieved
    while response.to_lowercase().trim() != "done"{
        println!("Please enter \"done\" when you have sumbitted openned the link and accepted permissions");
        response = String::new();
        stdin().read_line(&mut response).expect("Did not expect that string");
    }
    
    // Close the javascript server
    // TODO: Close specific "server.js" server
    Command::new("killall")
    .arg("node")
    .spawn()
    .expect("Failed to kill");

    let code  = fs::read_to_string("code.txt").expect("Could not read file"); 
    fs::remove_file("code.txt").expect("Failed to delete file"); // Remove file with code for security purposes
    code // Return authorization code
}

async fn get_current_track(auth_token: &str) -> Track{
    let url = format!("https://api.spotify.com/v1/me/player");

    let client = reqwest::Client::new(); // Client to handle API request
    let response = client
    .get(url)
    .header(AUTHORIZATION, format!("Bearer {auth_token}")) // Authorization based on token
    .header(ACCEPT, "application/json") // Recieve json response
    .send()
    .await
    .unwrap();
    
    match response.status(){
        reqwest::StatusCode::OK => {
            match response.json::<Player>().await {
                Ok(parsed) => {
                    return parsed.item // Return Track struct
                }
                Err(e) => panic!("the response did not match the struct {:?}", e)
            }
        },
        reqwest::StatusCode::UNAUTHORIZED => {
            panic!("new authentication code needed")
        },
        reqwest::StatusCode::NO_CONTENT => {
            panic!("No current song playing") // TODO: Handle without panic
        },
        other => panic!("there was an unexpected error: {:?}", other)
    }
}

#[tokio::main]
async fn main() {
    let mut is_new = String::new(); // Stores whether the current user is a new login
    let mut username = String::new(); // Stores the username value
    
    println!("Is this your first time logging in? (y/n)");
    let _ = stdout().flush();
    io::stdin().read_line(&mut is_new).expect("Did not expect that string"); // Store response

    let is_new = match is_new.chars().nth(0){ // Convert to boolean value based on response
        Some(response) => {
        if response == 'y' {
            true
        } else if  response == 'n' {
            false
        } else {
            panic!("Invalid response") // Neither y or n input
        }
    },
        None => panic!("String is empty")
    };

    let auth_token: String;
    if is_new { // For new user logins
        println!("Please input your spotify username: ");
        let _ = stdout().flush();
        io::stdin().read_line(&mut username).expect("Did not expect that string"); // Store to username variable
        username = String::from(username.trim()); // Trim to eliminate unwanted spaces or return keys
        let code: String = retrieve_code(); // Retrieve authorization code needed for generating the first token
        auth_token = generate_refresh(&username ,code).await; // Generate refresh token, and store in users.json
    } else { // For users with existing user record
        let file: Users = serde_json::from_str(
            fs::read_to_string("users.json")
            .expect("Error opening file")
            .as_str()
        ).expect("Could not convert to json");
        let user_list: Vec<String> = file.users.iter().map(|u| u.name.clone()).collect(); // Parse json file and generate a list of usernames
        
        // Print simple list for user selection
        println!("Users logged in: ");
        for i in 0..user_list.len(){
            println!("{}. {}", i+1, user_list.get(i).expect("Not allowed"))
        }

        let mut response = String::new(); // Store user response to list selection
        let _ = stdout().flush();
        io::stdin().read_line(&mut response).expect("Did not expect that string");
        response = response.trim().to_string();

        let response = match response.parse::<i32>(){
            Ok(n) => {
                if n > 0 {
                    n
                } else {
                    panic!("Invalid input, please enter integer value more than 0")
                }
            },
            Err(_) if user_list.contains(&response) => {
                // User responds with username
                username = response;
                -1
            },
            Err(_) => panic!("Not a valid input")
        };

        // If response was a number
        if response > 0 {
            username = user_list.get((response-1) as usize).unwrap().clone();
        }
        auth_token = generate_token(&username).await; // Generate token from refresh
    }

    let current_track = get_current_track(auth_token.as_str()).await; // Request current track user is playing

    let track_name = current_track.name;
    // Create String out of list of artists
    let artists: String = current_track.artists
    .iter().map(|a| a.name.clone())
    .collect::<Vec<String>>()
    .join(", ");

    println!("Current track is: {} by {}", track_name.trim(), artists);
}

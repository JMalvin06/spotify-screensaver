use reqwest::{ self, header::{ ACCEPT, CONTENT_TYPE } };
use serde::{ Serialize, Deserialize };
use url_search_params;
use open;
use tokio::{self, select, time::sleep};

use core::panic;
use std::{time::Duration};
use std::collections::HashMap;
use std::fs;

use tokio::io::{AsyncReadExt, AsyncWriteExt};


const URI: &str = "http://127.0.0.1:8000/callback";

/// Represents a refresh token response
#[derive(Serialize, Deserialize, Debug)]
struct AuthResponse {
    access_token: String,
    refresh_token: String,
}

/// Represents an access token response
#[derive(Serialize, Deserialize, Debug)]
struct Access {
    access_token: String,
}


/// Represents a json file holding the refresh token
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthFile {
    refresh: String,
}

/// Represents a response for user data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    display_name: String
}

/// Represents a json file holding client id and secret
#[derive(Serialize, Deserialize, Clone)]
pub struct Constants{
    id: String,
    secret: String 
}

/// Represents a client that can send requests to the Spotify API
#[derive(Default, Clone)]
pub struct SpotifyUser {
    /// Account display name
    username: String,
    /// User account's access token
    token: String,
    /// Spotify app client ID
    id: String,
    /// Spotify app client secret
    secret: String
}

impl SpotifyUser {
    /// Sets the client ID
    pub fn set_id(&mut self, id: &String) {
        self.id = id.to_string();
    }
    
    /// Sets the client secret
    pub fn set_secret(&mut self, secret: &String) {
        self.secret = secret.to_string();
    }

    #[tokio::main]
    pub async fn set_username(&mut self) {
        let url = format!("https://api.spotify.com/v1/me");

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .bearer_auth(&self.token)
            .header(ACCEPT, "application/json") // Recieve json response
            .send()
            .await
            .unwrap();

        match response.status() {
            reqwest::StatusCode::OK => {
                let res = response.text().await.unwrap();
                match serde_json::from_str::<User>(&res) {
                    Ok(user) => {
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

    pub(crate) fn get_username(&self) -> &str {
        return &self.username
    }
    
    /// Generates access token for account specific data request
    #[tokio::main]
    pub(crate) async fn generate_token(&mut self){
        // Convert user.json to struct format
        let file: AuthFile = serde_json
            ::from_str(fs::read_to_string("user.json").expect("Error opening file").as_str())
            .expect("Could not convert to json");
        // Retrieves refresh token
        let refresh = String::from(file.refresh);
        let auth_url = "https://accounts.spotify.com/api/token";

        let client = reqwest::Client::new();
        // Body parameters for API call
        let params = HashMap::from([
            (String::from("grant_type"), String::from("refresh_token")),
            (String::from("refresh_token"), refresh),
        ]);
        
        // Generate response for access token
        let response = client
            .post(auth_url)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .basic_auth(&self.id, Some(&self.secret)) // Authorize based on client ID and secret
            .body(url_search_params::build_url_search_params(params))
            .send().await
            .unwrap();

        // Listen for response from API
        match response.status() {
            reqwest::StatusCode::OK => {
                // Convert response json to struct
                match response.json::<Access>().await {
                    Ok(parsed) => {
                        self.token = parsed.access_token; // Recieve access token
                    }
                    Err(e) => {
                        println!("There was an error: {e}");
                        self.token = String::new(); // Set token to empty to avoid using a problematic token
                    }, // Likely invalid authorization code
                }
            }
            other => {
                panic!("Could not generate token: {:?}", other)
            },
        }
    }

    /// Prompts user to accept app permisions and retrieves authorization code from callback response
    async fn retrieve_auth(&self) -> String {
        // Open listener at callback URI
        let listener =  tokio::net::TcpListener::bind("127.0.0.1:8000").await.expect("Could not bind");
        
        // Scope of user data that application can access
        let scope = "user-read-private user-read-email user-read-playback-state";
        // Parameters for body of URL link
        let params = HashMap::from([
            (String::from("response_type"), String::from("code")),
            (String::from("client_id"), self.id.to_string()),
            (String::from("scope"), String::from(scope)),
            (String::from("redirect_uri"), String::from(URI)),
        ]);

        // Format redirect URL
        let url_out = format!(
            "https://accounts.spotify.com/authorize?{}",
            url_search_params::build_url_search_params(params)
        );

        println!("Opening {} on default browser", url_out);
        open::that(url_out).expect("Not a valid URL"); // Automatically open browser at link

        // Handles user response, times out after 20 seconds
        let handle = tokio::spawn(async move{
            select! {
                accepted = listener.accept() => {
                    match accepted {
                        Ok((mut stream, _addr)) => {
                            let mut buffer = [0; 512];
                            // Put response into buffer
                            stream.read(&mut buffer).await.unwrap();
                            // Check if buffer is correcyl formatted with authorization response
                            if buffer.starts_with(b"GET /callback?code="){
                                // Exctract code from buffer
                                let code = String::from_utf8_lossy(&buffer[19..275]).to_string();

                                // Construct response
                                let status_line = "HTTP/1.1 200 OK";
                                let contents = include_str!("response.html");
                                let length = contents.len();
                                let response = format!("{status_line}\r\nContent-Type: text/html\r\nContent-Length: {length}\r\n\r\n{contents}");

                                // Write and push html response
                                stream.write_all(response.as_bytes()).await.expect("Unable to write");
                                stream.flush().await.expect("Unable to send request");
                                std::thread::sleep(std::time::Duration::from_millis(1000));
                                return code;
                            }
                            panic!("unexpected error");
                        },
                        Err(e) => {
                            panic!("unexpected error: {:?}", e);
                        }

                    }
                },
                // Times out after 20s
                _ = sleep(Duration::from_secs(20)) => {
                    println!("timed out");
                    return String::from("408");
                }
            }
        });
        
        // Returns access token
        return handle.await.unwrap();
    }

    /// Generates refresh token, needed in order to generate access token
    /// 
    /// Returns `true` if refresh token was successfully generated, `false` otherwise
    #[tokio::main]
    pub(crate) async fn generate_refresh(&self) -> bool {
        let code = self.retrieve_auth().await;
        let auth_url = "https://accounts.spotify.com/api/token";

        // Parameters for body of API call
        let params = HashMap::from([
            (String::from("grant_type"), String::from("authorization_code")),
            (String::from("code"), String::from(&code)),
            (String::from("redirect_uri"), String::from(URI.trim())),
        ]);

        let client = reqwest::Client::new(); // Initialize client to handle API call
        let response = client
            .post(auth_url)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .basic_auth(&self.id, Some(&self.secret))
            .body(url_search_params::build_url_search_params(params))
            .header(ACCEPT, "application/json") // Recieve response in json format
            .timeout(std::time::Duration::from_millis(2000))
            .send().await
            .unwrap();

        // Listen for response
        match response.status() {
            reqwest::StatusCode::OK => {
                // Convert response json to struct
                match response.json::<AuthResponse>().await {
                    Ok(parsed) => {
                        // Retrieve refresh token from response
                        let refresh_token = parsed.refresh_token;

                        // Create json file with refresh token
                        let file: AuthFile = AuthFile {
                            refresh: String::from(refresh_token),
                        };
                        fs::write(
                            "user.json",
                            serde_json::to_string_pretty(&file).expect("Could not convert")
                        ).expect("Could not write");
                        let constants_file: Constants = Constants{ 
                            id: self.id.clone(), 
                            secret: self.secret.clone()
                        };
                        // Create json file with client ID and secret
                        fs::write(
                            "constants.json", 
                        serde_json::to_string_pretty(&constants_file).expect("could not convert")).expect("Failed to write to constants.json");
                        return true
                    }
                    Err(e) => panic!("the response did not match the struct {:?}", e),
                }
            }
            other => {
                println!("There was an unexpected error: {}", other);
                return false
            }
        }
    }

}
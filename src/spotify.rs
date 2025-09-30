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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Auth {
    refresh: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    display_name: String
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Constants{
    id: String,
    secret: String 
}

#[derive(Default, Clone)]
pub struct SpotifyUser {
    pub(crate) username: String,
    token: String,
    id: String,
    secret: String
}

impl SpotifyUser {
    pub fn set_id(&mut self, id: &String) {
        self.id = id.to_string();
    }

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
    
    #[tokio::main]
    pub(crate) async fn generate_token(&mut self){
        // Convert user.json to struct format
        let file: Auth = serde_json
            ::from_str(fs::read_to_string("user.json").expect("Error opening file").as_str())
            .expect("Could not convert to json");

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
            .send().await
            .unwrap();
        match response.status() {
            reqwest::StatusCode::OK => {
                match response.json::<Access>().await {
                    Ok(parsed) => {
                        self.token = parsed.access_token; // Recieve access token
                    }
                    Err(e) => {
                        println!("There was an error: {e}");
                        self.token = String::new();
                    }, // Likely invalid authorization code
                }
            }
            other => {
                println!("Could not generate token: {:?}", other)
            },
        }
    }

    async fn retrieve_code(&self) -> String {
        let scope = "user-read-private user-read-email user-read-playback-state";
        // Parameters for body of URL link
        let params = HashMap::from([
            (String::from("response_type"), String::from("code")),
            (String::from("client_id"), self.id.to_string()),
            (String::from("scope"), String::from(scope)),
            (String::from("redirect_uri"), String::from(URI)),
        ]);

        // Open listener at URI address
        let listener =  tokio::net::TcpListener::bind("127.0.0.1:8000").await.expect("Could not bind");

        let url_out = format!(
            "https://accounts.spotify.com/authorize?{}",
            url_search_params::build_url_search_params(params)
        );
        println!("Opening {} on default browser", url_out);
        open::that(url_out).expect("Not a valid URL"); // Automatically open browser at link

        let handle = tokio::spawn(async move{
            select! {
                accepted = listener.accept() => {
                    match accepted {
                        Ok((mut stream, _addr)) => {
                            //let mut req = &stream;
                            let mut buffer = [0; 512];
                            stream.read(&mut buffer).await.unwrap();
                            if buffer.starts_with(b"GET /callback?code="){
                                let code = String::from_utf8_lossy(&buffer[19..275]).to_string();
                                let status_line = "HTTP/1.1 200 OK";
                                let contents = include_str!("response.html");
                                let length = contents.len();
                                let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

                                // Write and push html response
                                stream.write_all(response.as_bytes()).await.expect("Unable to write");
                                stream.flush().await.expect("Unable to send request");
                                return code;
                            }
                            panic!("unexpected error");
                        },
                        Err(e) => {
                            panic!("unexpected error: {:?}", e);
                        }

                    }
                },
                _ = sleep(Duration::from_secs(5)) => {
                    println!("timed out");
                    return String::from("400");
                }
            }
        });

        return handle.await.unwrap();
    }

    #[tokio::main]
    pub(crate) async fn generate_user(&self) -> bool {
        let code = self.retrieve_code().await;
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
                            id: self.id.clone(), 
                            secret: self.secret.clone()
                        };
                        fs::write(
                            "constants.json", 
                        serde_json::to_string_pretty(&constants_file).expect("could not convert")).expect("Failed to write to constants.json");
                        return true
                    }
                    Err(e) => panic!("the response did not match the struct {:?}", e),
                }
            }
            other => {
                println!("{}, {}", other, code);
                return false
            }
        }
    }

}
use std::{fs, process::Command, time::Duration};

use iced::{
    alignment::Vertical::Top, time, widget::{ button, container, image::Handle, row, text, text_input, Container }, window::{self}, Alignment::Center, Length, Subscription, Task
};
use iced::widget::column;
use iced::widget::image;
use rfd::FileDialog;

use crate::spotify::{SpotifyUser, Track};
mod spotify;




enum Status {
    UserSelect,
    SignIn,
    CurrentTrack,
}
impl Default for Status {
    fn default() -> Self {
        Status::SignIn
    }
}

enum State {
    Idle,
    Refreshing,
}
impl Default for State {
    fn default() -> Self {
        State::Idle
    }
}

#[derive(Clone, Debug)]
enum Message {
    NextPage,
    InputID(String),
    InputSecret(String),
    ToSelection,
    RefreshTrack(()),
}

struct LoginMenu {
    client: SpotifyUser,
    cached_track: Track,
    cached_image_data: Handle,
    content: Status,
    id_input: String,
    secret_input: String,
    state: State,
}





// TODO add multithreaded support to not block app logic
impl LoginMenu {
    fn title(&self) -> String {
        String::from("User Menu")
    }

    fn new() -> (LoginMenu, Task<Message>) {
        (
            Self {
                client: SpotifyUser::new(),
                cached_track: spotify::Track::default(),
                cached_image_data: Handle::from_path("images/placeholder.jpg".to_string()),
                content: Status::default(),
                id_input: String::default(),
                secret_input: String::default(),
                state: State::Idle,
            },
            Task::none(),
        )
    }

    fn view(&self) -> Container<'_, Message> {
        match self.content {
            Status::UserSelect => {
                container(
                    column![
                        text(format!("Would you like to create a screensaver for {}?", self.client.username)).size(15),
                        button("Confirm").on_press(Message::NextPage)
                    ].align_x(Center)
                )
                .height(Length::Fill)
                .width(Length::Fill)
                .align_x(Center)
                .align_y(Center)
                .padding(10)
            }
            Status::SignIn => {
                container(
                    column![
                        text_input("Client ID", &self.id_input)
                        .width(250)
                        .on_input(|value| Message::InputID(value)),
                        text_input("Client Secret", &self.secret_input)
                            .width(250)
                            .on_input(|value| Message::InputSecret(value))
                            .on_submit(Message::ToSelection),
                        row![
                            button("Submit").on_press(Message::ToSelection)
                        ].spacing(30)
                    ].align_x(Center)
                )
                    .height(Length::Fill)
                    .width(Length::Fill)
                    .align_x(Center)
                    .align_y(Center)
                    .padding(20)
            }

            Status::CurrentTrack => {
                let current_track = self.client.clone().get_track();
                let current_artists = current_track.artists
                    .iter()
                    .map(|a| a.name.clone())
                    .collect::<Vec<String>>()
                    .join(", ");
                
                container(
                        column![
                            text("Current Track").size(50),
                            image::viewer(self.cached_image_data.clone()).height(Length::Fixed(300.0)),
                            text(current_track.name).size(40),
                            text(current_artists).size(25),
                        ]
                            .spacing(30)
                            .align_x(Center)
                    )
                        .height(Length::Fill)
                        .width(Length::Fill)
                        .align_x(Center)
                        .align_y(Top)
            }
        }
    }

    fn get_album_art(&self) -> Handle {
        let current_image = self.client.clone().get_track().album.images[0].clone();
        if current_image.url.contains("placeholder.jpg") {
            return Handle::from_path(current_image.url);
        } else {
            println!("new album image: {}", current_image.url);
            return Handle::from_bytes(self.client.clone().get_image_data());
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let tick: Subscription<Message> = match self.state {
            State::Idle => Subscription::none(),
            State::Refreshing { .. } => {
                let t = time
                    ::every(Duration::from_millis(1000))
                    .map(|_arg0: std::time::Instant| Message::RefreshTrack(()));
                return t;
            }
        };

        return tick;
    }

    fn refresh_track(&mut self) {
        if self.client.clone().token_empty(){
            self.client.generate_token();
        }
        self.client.refresh_track();
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::NextPage => {
                //if self.selection != String::from("Select a user.."){
                     
                    let user_file = "user.json";
                    let constants_file = "constants.json";
                    let destination = FileDialog::new().pick_folder();
                    let path = destination.clone().unwrap().as_path().to_path_buf();
                    let mut user_path = path.clone();
                    user_path.push("SpotifyScreensaver/user.json");
                    let mut constants_path = path.clone();
                    constants_path.push("SpotifyScreensaver/constants.json");
                    fs::copy(user_file,user_path).expect("Unable to copy file to resources");
                    fs::copy(constants_file,constants_path).expect("Unable to copy file to resources");


                    Command::new("xcodebuild").current_dir(path).arg("build").output().expect("Could not build");
                    self.refresh_track();
                    self.cached_image_data = self.get_album_art();
                    self.cached_track = self.client.clone().get_track().clone();
                    self.state = State::Refreshing;
                    self.content = Status::CurrentTrack;
                //}
            }
            Message::InputID(value) => {
                self.id_input = value;
            }
            Message::InputSecret(value) => {
                self.secret_input = value;
            }
            Message::ToSelection => {
                self.client.set_id(self.id_input.clone());
                self.client.set_secret(self.secret_input.clone());
                self.client.clone().generate_user();
                self.client.generate_token();
                self.client.set_username();

                self.content = Status::UserSelect;
            }
            Message::RefreshTrack(_) => {
                self.refresh_track();
                let current_track = self.client.clone().get_track();
                if current_track != self.cached_track {
                    self.cached_image_data = self.get_album_art();
                    self.cached_track = current_track.clone();
                } 
            }
        }
    }
}


fn main() -> iced::Result {
    let window_settings = window::Settings {
        size: iced::Size { width: 450.0, height: 200.0},
        resizable: true, 
        ..Default::default()
    };
    let app = iced
        ::application(LoginMenu::title, LoginMenu::update, LoginMenu::view)
        .subscription(LoginMenu::subscription)
        .window(window_settings);
    app.run_with(LoginMenu::new)
}

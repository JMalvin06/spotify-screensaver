use std::{fs, process::Command, slice::Windows, str, time::Duration};

use iced::{
    alignment::Vertical::Top, time, widget::{ button, container, image::Handle, row, text, text_input, Container }, window::{self, Id}, Alignment::Center, Length, Settings, Subscription, Task
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
    SelectedUser(String),
    NextPage,
    SignIn,
    InputValue(String),
    ToSelection,
    RefreshTrack(()),
    DeleteUser
}

struct LoginMenu {
    client: SpotifyUser,
    cached_track: Track,
    cached_image_data: Handle,
    selection: String,
    content: Status,
    input: String,
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
                selection: "Select a user..".to_string(),
                cached_track: spotify::Track::default(),
                cached_image_data: Handle::from_path("images/placeholder.jpg".to_string()),
                content: Status::default(),
                input: String::default(),
                state: State::Idle,
            },
            Task::none(),
        )
    }

    fn view(&self) -> Container<'_, Message> {
        match self.content {
            Status::UserSelect => {
                let user = spotify::get_user().name;

                container(
                    column![
                        text(format!("Would you like to create a screensaver with {user}?")).size(15),
                        button("Confirm").on_press(Message::NextPage)
                    ].align_x(Center)
                )
                .height(Length::Fill)
                .width(Length::Fill)
                .align_x(Center)
                .align_y(Center)
                .padding(10)
                /*
                let list = iced::widget::pick_list(
                    spotify::get_user_list(),
                    Some(self.selection.clone()),
                    Message::SelectedUser
                );

                container(
                    column![
                        text("User Login").size(50),
                        list.placeholder("Select a user.."),
                        column![
                            row![
                                button("Sign In").on_press(Message::SignIn),
                                button("Next").on_press(Message::NextPage)
                            ].spacing(50),
                            button("Delete User").on_press(Message::DeleteUser),
                        ].align_x(Center)
                        .spacing(10),
                    ]
                        .align_x(Center)
                        .spacing(20),
                )
                    .height(Length::Fill)
                    .width(Length::Fill)
                    .align_x(Center)
                    .align_y(Center)
                    .padding(100)
                    */
            }
            Status::SignIn => {
                container(
                    column![
                        text_input("Spotify Username", &self.input)
                            .width(250)
                            .on_input(|value| Message::InputValue(value))
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
            Message::SelectedUser(value) => {
                self.selection = value;
            }
            Message::NextPage => {
                //if self.selection != String::from("Select a user.."){
                     
                    let file = "user.json";
                    let destination = FileDialog::new().pick_folder();
                    let path = destination.clone().unwrap().as_path().to_path_buf();
                    let mut new_path = path.clone();
                    new_path.push("SpotifyScreensaver/user.json");
                    fs::copy(file,new_path).expect("Unable to copy file to resources");

                    Command::new("xcodebuild").current_dir(path).arg("build").output().expect("Could not build");
                    self.refresh_track();
                    self.cached_image_data = self.get_album_art();
                    self.cached_track = self.client.clone().get_track().clone();
                    self.state = State::Refreshing;
                    self.content = Status::CurrentTrack;
                //}
            }
            Message::SignIn => {
                self.content = Status::SignIn;
            }
            Message::InputValue(value) => {
                self.input = value;
            }
            Message::ToSelection => {
                spotify::generate_user(self.input.clone(), self.client.clone());

                self.content = Status::UserSelect;
            }
            Message::RefreshTrack(_) => {
                self.refresh_track();
                let current_track = self.client.clone().get_track();
                if current_track != self.cached_track {
                    self.cached_image_data = self.get_album_art();
                    self.cached_track = current_track.clone();
                } 
            },
            Message::DeleteUser => {
                //spotify::delete_user(self.selection.clone());
                self.selection = String::from("Select a user..");
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

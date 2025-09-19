use std::time::Duration;

use iced::{
    alignment::Vertical::Top,
    time,
    widget::{ button, container, image::Handle, row, text, text_input, Container },
    Alignment::Center,
    Length,
    Subscription,
    Task,
};
use iced::widget::column;
use iced::widget::image;

use crate::spotify::SpotifyUser;
mod spotify;



enum Status {
    UserSelect,
    SignIn,
    CurrentTrack,
}
impl Default for Status {
    fn default() -> Self {
        Status::UserSelect
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
    Cancel,
    DeleteUser
}

#[derive(Default)]
struct LoginMenu {
    client: SpotifyUser,
    selection: String,
    content: Status,
    input: String,
    state: State,
}


impl LoginMenu {
    fn title(&self) -> String {
        String::from("User Menu")
    }

    fn new() -> (LoginMenu, Task<Message>) {
        (
            Self {
                client: SpotifyUser::new(),
                selection: "Select a user..".to_string(),
                content: Status::UserSelect,
                input: String::default(),
                state: State::Idle,
            },
            Task::none(),
        )
    }

    fn view(&self) -> Container<'_, Message> {
        match self.content {
            Status::UserSelect => {
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
            }
            Status::SignIn => {
                container(
                    column![
                        text_input("Spotify Username", &self.input)
                            .width(250)
                            .on_input(|value| Message::InputValue(value))
                            .on_submit(Message::ToSelection),
                        row![
                            button("Cancel").on_press(Message::Cancel),
                            button("Submit").on_press(Message::ToSelection)
                        ].spacing(30)
                    ].align_x(Center)
                )
                    .height(Length::Fill)
                    .width(Length::Fill)
                    .align_x(Center)
                    .align_y(Center)
                    .padding(300)
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
                        image::viewer(self.get_album_art()).height(Length::Fixed(300.0)),
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
        println!("Check");
        let current_image = self.client.clone().get_track().album.images[0].clone();
        if current_image.url.contains("placeholder.jpg") {
            return Handle::from_path(current_image.url);
        } else {
            println!("{}", current_image.url);
            return Handle::from_bytes(self.client.clone().get_image_data());
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let tick: Subscription<Message> = match self.state {
            State::Idle => Subscription::none(),
            State::Refreshing { .. } => {
                let t = time
                    ::every(Duration::from_millis(5000))
                    .map(|_arg0: std::time::Instant| Message::RefreshTrack(()));
                return t;
            }
        };

        return tick;
    }

    fn refresh_track(&mut self) {
        if self.client.clone().token_empty(){
            self.client.generate_token(self.selection.clone());
        }
        self.client.refresh_track();
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::SelectedUser(value) => {
                self.selection = value;
            }
            Message::NextPage => {
                if self.selection != String::from("Select a user.."){
                    self.refresh_track();
                    self.state = State::Refreshing;
                    self.content = Status::CurrentTrack;
                }
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
            }
            Message::Cancel => {
                self.content = Status::UserSelect;
            },
            Message::DeleteUser => {
                spotify::delete_user(self.selection.clone());
                self.selection = String::from("Select a user..");
            }
        }
    }
}


fn main() -> iced::Result {
    let app = iced
        ::application(LoginMenu::title, LoginMenu::update, LoginMenu::view)
        .subscription(LoginMenu::subscription);
    app.run_with(LoginMenu::new)
}

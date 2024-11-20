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
}

#[derive(Default)]
struct LoginMenu {
    selection: String,
    token: String,
    content: Status,
    input: String,
    current_track: String,
    current_artist: String,
    current_image: String,
    state: State,
}

impl LoginMenu {
    fn title(&self) -> String {
        String::from("User Menu")
    }

    fn new() -> (LoginMenu, Task<Message>) {
        (
            Self {
                selection: String::from("Select a user.."),
                token: String::new(),
                content: Status::UserSelect,
                input: String::default(),
                current_track: String::default(),
                current_artist: String::default(),
                current_image: String::from("images/placeholder.jpg"),
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
                        row![
                            button("Sign In").on_press(Message::SignIn),
                            button("Next").on_press(Message::NextPage)
                        ].spacing(50)
                    ]
                        .align_x(Center)
                        .spacing(40)
                )
                    .height(Length::Fill)
                    .width(Length::Fill)
                    .align_x(Center)
                    .align_y(Center)
                    .padding(100)
                //.align_y(Center)
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
                container(
                    column![
                        text("Current Track").size(50),
                        image::viewer(self.get_album_art()).height(Length::Fixed(300.0)),
                        text(self.current_track.clone()).size(40),
                        text(self.current_artist.clone()).size(25)
                        /*button("Refresh")
                        .on_press(Message::RefreshTrack)*/
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
        if self.current_image.contains("placeholder.jpg") {
            return Handle::from_path(self.current_image.clone());
        } else {
            return Handle::from_bytes(spotify::get_image(self.current_image.clone()));
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
        if self.token.is_empty() {
            self.token = spotify::generate_token(self.selection.clone());
        }
        let track = spotify::get_current_track(&self.token);
        if track.name.is_empty() && track.artists.is_empty() {
            self.current_track = String::from("No song playing");
            self.current_artist = String::from("N/A");
            self.current_image = String::from("images/placeholder.jpg");
        } else {
            self.current_track = track.name;
            self.current_artist = track.artists
                .iter()
                .map(|a| a.name.clone())
                .collect::<Vec<String>>()
                .join(", ");
            self.current_image = match track.album.images.get(0) {
                Some(i) => { i.url.clone() }
                None => { String::from("images/placeholder.jpg") }
            };
        }
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::SelectedUser(value) => {
                self.selection = value;
            }
            Message::NextPage => {
                self.refresh_track();
                self.state = State::Refreshing;
                self.content = Status::CurrentTrack;
            }
            Message::SignIn => {
                self.content = Status::SignIn;
            }
            Message::InputValue(value) => {
                self.input = value;
            }
            Message::ToSelection => {
                //println!("User: {}", self.input);
                let user = self.input.clone();
                let code = spotify::retrieve_code();
                spotify::generate_refresh(user, code);

                self.content = Status::UserSelect;
            }
            Message::RefreshTrack(_) => {
                self.refresh_track();
                println!("{}", self.current_image);
            }
            Message::Cancel => {
                self.content = Status::UserSelect;
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

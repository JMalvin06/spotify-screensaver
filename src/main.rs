use core::fmt;

use iced::{widget::{button, container, row, text, text_input, Container}, Alignment::Center, Length, Task};
use iced::widget::column;
mod spotify;

#[derive(Default)]
struct LoginMenu{
    selection: String,
    token: String,
    content: Status,
    input: String,
    current_track: String
}

#[derive(Clone, Debug)]
enum Message{
    SelectedUser(String),
    NextPage,
    SignIn,
    InputValue(String),
    ToSelection,
    RefreshTrack
}

enum Status{
    UserSelect,
    SignIn,
    CurrentTrack
}
impl Default for Status {
    fn default() -> Self { Status::UserSelect }
}


impl LoginMenu{
    
    fn title(&self) -> String{
        String::from("User Menu")
    }

    fn new() -> (LoginMenu, Task<Message>){
        (Self{
            selection: String::from("Select a user.."),
            token: String::new(),
            content: Status::UserSelect,
            input: String::default(),
            current_track: String::from("None")
        }, Task::none())
    }

    fn view(&self) -> Container<'_, Message>{
        match self.content {
            Status::UserSelect => {
                let list  = iced::widget::pick_list(spotify::get_user_list(), Some(self.selection.clone()), Message::SelectedUser);
                container(
                    column![
                    text("User Login")
                    .size(50),
                    list
                    .placeholder("Select a user.."),
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
                        button("Submit")
                        .on_press(Message::ToSelection)
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
                        text("Current Track")
                        .size(20),
                        text(self.current_track.clone())
                        .size(25),
                        button("Refresh")
                        .on_press(Message::RefreshTrack)
                    ]
                    .spacing(10)
                    .align_x(Center)
                )
                .height(Length::Fill)
                .width(Length::Fill)
                .align_x(Center)
                .align_y(Center)
                .padding(300)
            }
        }
    }
    

    fn update(&mut self, message: Message){
        match message{
            Message::SelectedUser(value) => {
                self.selection = value;
            }
            Message::NextPage => {
                println!("Task Start");
                self.token = spotify::generate_token(self.selection.clone());
                let track = spotify::get_current_track(&self.token);
                self.current_track = track.name;
                self.content = Status::CurrentTrack;
            }
            Message::SignIn => {
                self.content = Status::SignIn
            }
            Message::InputValue(value) => self.input = value,
            Message::ToSelection => {
                //println!("User: {}", self.input);
                let user = self.input.clone();
                let code = spotify::retrieve_code();
                spotify::generate_refresh(user, code);

                self.content = Status::UserSelect
            }
            Message::RefreshTrack => {
                self.token = spotify::generate_token(self.selection.clone());
                let track = spotify::get_current_track(&self.token);
                self.current_track = track.name;
            }
        }
    }
}


fn main() -> iced::Result{
    let app = iced::application(LoginMenu::title, LoginMenu::update, LoginMenu::view);
    app.run_with(LoginMenu::new)
}

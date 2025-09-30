use std::{fs, path::{Path, PathBuf}, process::Command};

use iced::{
    alignment::{Horizontal::{self}}, widget::{ button, container, row, text, text_input, Container }, window::{self}, Alignment::Center, Color, Font, Length, Task
};
use iced::widget::column;
use rfd::FileDialog;

use crate::spotify::{SpotifyUser};
mod spotify;




enum Status {
    UserSelect,
    SignIn,
    SuccessPage,
}
impl Default for Status {
    fn default() -> Self {
        Status::SignIn
    }
}

#[derive(Clone, Debug)]
enum Message {
    NextPage,
    InputID(String),
    InputSecret(String),
    ToSelection,
    SelectBuild,
    SelectOutput,
    CloseWindow
}


struct LoginMenu {
    client: SpotifyUser,
    content: Status,
    id_input: String,
    secret_input: String,
    build_dir: PathBuf,
    build_status: (String, bool),
    output_dir: PathBuf,
    output_status: (String, bool),
    sign_in_message: String
}

impl Default for LoginMenu {
    fn default() -> Self {
        Self { 
            client: Default::default(), 
            content: Default::default(), 
            id_input: Default::default(), 
            secret_input: Default::default(), 
            build_dir: {
                let parent_dir = std::env::current_exe()
                                          .unwrap()
                                          .parent()
                                          .expect("Cannot find parent")
                                          .join(Path::new("SpotifyScreensaver"));
                if containts_valid(&parent_dir) {
                    parent_dir
                } else {
                    Default::default()
                }
            }, 
            build_status: (String::default(), false),
            output_dir: std::env::current_exe().unwrap().parent().expect("Cannot find parent").to_path_buf(),
            output_status: (String::default(), false),
            sign_in_message: String::default()
        }
    }
}





impl LoginMenu {
    fn title(&self) -> String {
        String::from("User Menu")
    }

    fn new() -> (LoginMenu, Task<Message>) {
        (
            Self::default(),
            Task::none(),
        )
    }

    fn view(&self) -> Container<'_, Message> {
        match self.content {
            Status::UserSelect => {
                let build_red = if self.build_status.1 {Color::from_rgb(255.0, 255.0,255.0)} else {Color::from_rgb(100.0, 0.0, 0.0)};
                container(
                    column![
                        text(format!("Successfully found account: {}", self.client.username)).size(15),

                        row![
                            text("Build folder: ").size(15),
                            text(&self.build_status.0).size(15).color(build_red)
                        ].width(Length::Fixed(300.0)),
                        row![
                            text_input("Build Directory", &self.build_dir.to_str().expect("Could not convert")),
                            button("...").on_press(Message::SelectBuild)
                        ].width(Length::Fixed(300.0)),

                        row![
                            text("Output folder: ").size(15),
                        ].width(Length::Fixed(300.0)),
                        row![
                            text_input("Output Directory", &self.output_dir.to_str().expect("Could not convert")),
                            button("...").on_press(Message::SelectOutput)
                        ].width(Length::Fixed(300.0)),

                        button("Confirm").on_press(Message::NextPage)
                    ].align_x(Horizontal::Center)
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
                        ].spacing(30),
                        text(&self.sign_in_message).color(Color::from_rgb(255.0, 0.0, 0.0))
                    ].align_x(Center)
                )
                    .height(Length::Fill)
                    .width(Length::Fill)
                    .align_x(Center)
                    .align_y(Center)
                    .padding(20)
            }
            Status::SuccessPage => {
                container(
                    column![
                        text("Success!")
                        .size(18)
                        .font(Font{weight: iced::font::Weight::Bold, ..Font::default()}),
                        text("You can now find the built screensaver in the output directory")
                        .size(16),
                        button("Close Installer").on_press(Message::CloseWindow)
                    ]
                    .align_x(Center)
                    .spacing(10)
                )
                .height(Length::Fill)
                .width(Length::Fill)
                .align_x(Center)
                .align_y(Center)
                .padding(10)
            },
                    }
    }


    fn update(&mut self, message: Message) {
        match message {
            Message::NextPage => {
                if self.build_dir.exists() {
                    let user_file = "user.json";
                    let constants_file = "constants.json";
                    let mut user_path = self.build_dir.clone();
                    user_path.push("SpotifyScreensaver/user.json");
                    let mut constants_path = self.build_dir.clone();
                    constants_path.push("SpotifyScreensaver/constants.json");
                    fs::copy(user_file,user_path).expect("Unable to copy file to resources");
                    fs::copy(constants_file,constants_path).expect("Unable to copy file to resources");

                    Command::new("xcodebuild").current_dir(&self.build_dir).arg("build").output().expect("Could not build");
                    
                    let saver_path = self.build_dir.clone().join(Path::new("build/Release/SpotifyScreensaver.saver"));
                    let  output_path: PathBuf = self.output_dir.clone().join(Path::new("SpotifyScreensaver.saver"));

                    if saver_path.exists() {
                        copy_dir(saver_path, output_path).expect("Could not copy saver to output directory");
                    } else {
                        panic!("Could not find file");
                    }
                    self.content = Status::SuccessPage;
                }
            }
            Message::InputID(value) => {
                self.id_input = value;
            }
            Message::InputSecret(value) => {
                self.secret_input = value;
            }
            Message::ToSelection => {
                self.client.set_id(&self.id_input);
                self.client.set_secret(&self.secret_input);
                println!("Empty? {}", self.id_input.is_empty());
                if !(self.id_input.is_empty() || self.secret_input.is_empty()) && self.client.generate_user(){
                    self.client.generate_token();
                    self.client.set_username();
                    self.content = Status::UserSelect;
                } else {
                    self.sign_in_message = String::from("Invalid ID or secret, please try again");
                }
            }
            Message::CloseWindow => {
                std::process::exit(0);
            }
            Message::SelectBuild => {
                let destination = FileDialog::new().pick_folder();
                if destination.is_some() {
                    let path = destination.as_ref().unwrap();
                    if containts_valid(path) {
                        self.build_dir = destination.unwrap();
                        self.build_status = (String::from("valid directory"), true);
                    } else {
                        self.build_status = (String::from("invalid directory"), false);
                    }
                } else {
                    self.build_status = (String::from("please select a valid folder"), false);
                }
            }
            Message::SelectOutput => {
                let destination = FileDialog::new().pick_folder();
                if destination.is_some() {
                    self.output_dir = destination.unwrap();
                    self.output_status = (String::from("valid directory"), true);
                } else {
                    self.output_status = (String::from("please select a valid folder"), false);
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
        .window(window_settings);
    app.run_with(LoginMenu::new)
}


fn copy_dir(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst).expect("Cannot Create Directory");
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn containts_valid(path: &Path) -> bool {
    return path.exists() && path.join(Path::new("SpotifyScreensaver.xcodeproj")).exists()
}
#![windows_subsystem = "windows"]
use std::{fs, io::BufRead, path::{Path, PathBuf}, process::{Command, Stdio}, sync::{Arc, Mutex}, time::Duration};

use std::io::BufReader;
use iced::{
    alignment::Horizontal::{self}, widget::{ button, container, progress_bar, row, text, text_input, Container }, window::{self}, Alignment::Center, Color, Font, Length, Subscription, Task
};
use iced::widget::column;
use rfd::FileDialog;

use crate::spotify::{SpotifyUser};
mod spotify;




enum Status {
    UserSelect,
    SignIn,
    Loading,
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
    CloseWindow,
    Tick
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
    sign_in_message: String,
    progress: Arc<Mutex<i32>>,
    progress_int: i32
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
                                          .join(Path::new("spotify_screensaver"));
                if containts_valid(&parent_dir) {
                    parent_dir
                } else {
                    Default::default()
                }
            }, 
            build_status: (String::default(), false),
            output_dir: std::env::current_exe().unwrap().parent().expect("Cannot find parent").to_path_buf(),
            output_status: (String::default(), false),
            sign_in_message: String::default(),
            progress: Arc::new(Mutex::new(0)),
            progress_int: 0
        }
    }
}

const DEP_COUNT: i32 = 239;

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
            Status::Loading => {
                container(
                    column![
                        text("Building your screensaver, please wait..")
                        .size(16)
                        .font(Font{weight: iced::font::Weight::Bold, ..Font::default()}),
                        progress_bar(0.0..=(DEP_COUNT-1) as f32, self.progress_int as f32),
                        text(format!("{}%", ((self.progress_int as f32/DEP_COUNT as f32)*100.0).round()))
                        .size(13)
                    ]
                    .align_x(Center)
                    .spacing(10)
                )
                .height(Length::Fill)
                .width(Length::Fill)
                .align_x(Center)
                .align_y(Center)
                .padding(10)
            }
            Status::SuccessPage => {
                container(
                    column![
                        text("Success!")
                        .size(18)
                        .font(Font{weight: iced::font::Weight::Bold, ..Font::default()}),
                        text("You can now find the built screensaver in the output directory")
                        .size(16),
                        button("Close Installer").on_press(Message::CloseWindow),
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
        let progress_clone = Arc::clone(&self.progress);
        let progress_count = progress_clone.lock().unwrap();


        match message {
            Message::NextPage => {
                        if self.build_dir.exists() {
                            let user_file = "user.json";
                            let constants_file = "constants.json";
                            let mut user_path = self.build_dir.clone();
                            user_path.push("src/user.json");
                            let mut constants_path = self.build_dir.clone();
                            constants_path.push("src/constants.json");
                            fs::copy(user_file,user_path).expect("Unable to copy file to resources");
                            fs::copy(constants_file,constants_path).expect("Unable to copy file to resources");

                            let mut cmd = Command::new("cargo").current_dir(&self.build_dir).arg("build").arg("--release").stderr(Stdio::piped()).spawn().unwrap();
                            let stdout = cmd.stderr.take().unwrap();
                            let reader = BufReader::new(stdout);
                            let progress_copy = Arc::clone(&self.progress);

                            std::thread::spawn(move || {
                                for _line in reader.lines() {
                                    let mut progress_copy = progress_copy.lock().expect("Could not unwrap progress");
                                    *progress_copy += 1;
                                    // println!("line: {:?}", line);
                                    std::thread::sleep(Duration::from_millis(1));
                                }

                                let mut progress_copy = progress_copy.lock().expect("Could not unwrap progress");
                                *progress_copy = DEP_COUNT;
                            });
                    

                            let saver_path = self.build_dir.clone().join(Path::new("target/release/spotify_screensaver").with_extension("exe"));
                            let  output_path: PathBuf = self.output_dir.clone().join(Path::new("spotify_screensaver").with_extension("scr"));

                            if saver_path.exists() && saver_path.is_file(){
                                fs::copy(saver_path, output_path).expect("Could not copy saver to output directory");
                            } else {
                                println!("Could not find file");
                            }
                            self.content = Status::Loading;
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
                        if !(self.id_input.is_empty() || self.secret_input.is_empty()) && self.client.generate_user(){
                            self.client.generate_token();
                            self.client.set_username();
                            self.content = Status::UserSelect;
                        } else {
                            let error = if self.id_input.is_empty() || self.secret_input.is_empty() {"empty client or secret"} else {"timed out"};
                            self.sign_in_message = String::from(format!("{}, please try again", error));
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
            Message::Tick => {
                match self.content {
                    Status::Loading => {
                        self.progress_int = *progress_count;

                        if self.progress_int >= DEP_COUNT {
                            self.content = Status::SuccessPage;
                        }
                    }
                    _ => {}
                }
            },
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(10)).map(|_| Message::Tick) 
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

fn containts_valid(path: &Path) -> bool {
    return path.exists() && path.join(Path::new("src")).exists()
}
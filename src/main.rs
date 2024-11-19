use iced::{alignment::Vertical::Top, widget::{container, text, Container}, Alignment::Center, Length, Task};
use iced::widget::column;
mod spotify;

#[derive(Default)]
struct LoginMenu{
    selection: String
}

#[derive(Clone, Debug)]
enum Message{
    SelectedUser(String)
}


impl LoginMenu{
    
    fn title(&self) -> String{
        String::from("User Menu")
    }

    fn new() -> (LoginMenu, Task<Message>){
        (Self{
            selection: String::from("Please select a user.."),
            //user_list: todo!(),
        }, Task::none())
    }

    fn view(&self) -> Container<'_, Message>{
        //let mut state: State<String> = State::new(vec![String::from("a"), String::from("b")]);
        let list  = iced::widget::pick_list(spotify::get_user_list(), Some(self.selection.clone()), Message::SelectedUser);
        
        container(
            column![
            text("User Login")
            .size(50),
            list
            .placeholder("Select a user..")
            ].align_x(Center)
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .align_x(Center)
        .align_y(Top)
        .padding(100)
        //.align_y(Center)
    }

    fn update(&mut self, message: Message){
        match message{
            Message::SelectedUser(value) => {
                self.selection = value
            }
        }
    }
}




fn main() -> iced::Result{
    let app = iced::application(LoginMenu::title, LoginMenu::update, LoginMenu::view);
    app.run_with(LoginMenu::new)
    //iced::run(LoginMenu::title, |arg0: &mut LoginMenu, message| LoginMenu::update(&*arg0, message) ,LoginMenu::view)
}

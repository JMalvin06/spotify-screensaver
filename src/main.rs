use std::sync::mpsc;

use bytes::Bytes;
use nannou::prelude::*;
mod spotify;

const SIZE: f32 = 200.;
const SPEED: f32 = 100.;

static mut CAN_RECIEVE: bool = false;


fn set_can_recieve(val: bool){
    unsafe { CAN_RECIEVE = val };
}

#[tokio::main]
async fn main() {
    nannou::app(model)
        .update(update)
        .run();
}

struct Model {
    x: f32,
    y: f32,
    last_time: f32,
    x_sign: f32,
    y_sign: f32,
    texture: wgpu::Texture,
    img_send: mpsc::Sender<Bytes>, 
    img_recieve: mpsc::Receiver<Bytes>, 
    token_send: mpsc::Sender<String>,
    token_recieve: mpsc::Receiver<String>,
    next_refresh: f32,
    window: WindowId,
    token: String
}

fn model(app: &App) -> Model {
    let window = app.new_window().fullscreen().view(view).build().unwrap();
    let (img_send, img_recieve) = mpsc::channel();
    let (token_send, token_recieve) = mpsc::channel();
    let assets = app.assets_path().unwrap();
    let img_path = assets.join("images").join("placeholder.jpg");
    let texture = wgpu::Texture::from_path(app, img_path).expect("Failed to load");
    
    Model {
        x: 0.0,
        y: 0.0,
        last_time: 0.0,
        x_sign: 1.0,
        y_sign: 1.0,
        texture: texture,
        img_recieve,
        img_send,
        token_send,
        token_recieve,
        next_refresh: 0.,
        token: String::default(),
        window
    }
}

fn update(app: &App, model: &mut Model, _update: Update) { 
    let boundary = app.window_rect();
    let delta_t = app.time - model.last_time;

    model.x += delta_t * SPEED * model.x_sign;
    model.y += delta_t * SPEED * model.y_sign;
    
    if model.x+SIZE/2. >= boundary.right() && model.x_sign > 0. {
        model.x_sign = -1.0;
    } else if model.x-SIZE/2. <= boundary.left() {
        model.x_sign = 1.0;
    }
    if model.y+SIZE/2. >= boundary.top() && model.y_sign > 0. {
        model.y_sign = -1.0;
    } else if model.y-SIZE/2. <= boundary.bottom() && model.y_sign < 0. {
        model.y_sign = 1.0;
    }

    model.last_time = app.time;

    
    if model.token.is_empty() {
        tokio::spawn(spotify::generate_token(model.token_send.clone(), String::from("JMalvin06")));
        model.token = String::from("None");
        model.token = match model.token_recieve.recv() {
            Ok(b) => {
                println!("Reponse!");
                b
            },
            Err(_) => {
                println!("None");
                String::default()
            }
        };
    }
    
    if app.time > model.next_refresh && unsafe { !CAN_RECIEVE }{
        println!("Sent");
        tokio::spawn(spotify::get_current_track(model.img_send.clone(),  model.token.clone()));
        model.next_refresh +=  5.;
    }


    if unsafe { CAN_RECIEVE } {
        let bytes = match model.img_recieve.try_recv() {
            Ok(p) => {
                println!("Recieved");
                p
            }
            Err(_) => {
                return;
            }
        };
        let album_art = nannou::image::load_from_memory(&bytes).expect("Unable to load");
        model.texture = wgpu::Texture::from_image(app, &album_art);
        println!("Loaded");
        unsafe { CAN_RECIEVE = false };
    }
}

fn view(app: &App, model: &Model, frame: Frame){  
    app.window(model.window).unwrap().set_cursor_visible(false);
    let draw = app.draw();
    frame.clear(BLACK);
    draw.texture(&model.texture)
    .x_y(model.x, model.y)
    .w_h(SIZE,SIZE);
    draw.to_frame(app, &frame).unwrap()
}

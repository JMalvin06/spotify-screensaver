#![windows_subsystem = "windows"]
use std::sync::{mpsc, Arc};

use bytes::Bytes;
use nannou::prelude::*;
use tokio::sync::Mutex;
use rand::Rng;

use crate::spotify::SpotifyUser;
mod spotify;

const SIZE: f32 = 300.;
const SPEED: f32 = 90.;




#[tokio::main]
async fn main() {
    nannou::app(model)
        .update(update)
        .event(event)
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
    next_refresh: f32,
    window: WindowId,
    cached_bytes: Bytes,
    client: Arc<Mutex<SpotifyUser>>,
    mouse_pos: Vec2,
    first_frame: bool
}

fn model(app: &App) -> Model {
    let window = app.new_window().fullscreen().visible(false).view(view).build().unwrap();
    let (img_send, img_recieve) = mpsc::channel();
    let assets = app.assets_path().unwrap();
    let img_path = assets.join("images").join("placeholder.png");
    let texture = wgpu::Texture::from_path(app, img_path).expect("Failed to load");
    let client = SpotifyUser::new();
    let rand_x = rand::thread_rng().gen_bool(0.5);
    let rand_y = rand::thread_rng().gen_bool(0.5);

    
    
    Model {
        x: 0.0,
        y: 0.0,
        last_time: 0.0,
        x_sign: if rand_x {1.0} else {-1.0},
        y_sign: if rand_y {1.0} else {-1.0},
        texture: texture,
        img_recieve,
        img_send,
        next_refresh: 0.,
        window,
        client: Arc::new(Mutex::new(client)),
        cached_bytes: Bytes::default(),
        mouse_pos: app.mouse.position(),
        first_frame: true
    }
}

fn event(app: &App, model: &mut Model, event: Event) {
    match event {
        Event::WindowEvent {id: _, simple: event } => {
            if event.is_some() {
                match event.unwrap() {
                    MouseMoved(pos) => {
                        if model.mouse_pos == Vec2::new(0.0, 0.0) {
                            model.mouse_pos = app.mouse.position();
                        } else if pos != model.mouse_pos {
                            std::process::exit(0);
                        }
                    },
                    _ => {}
                }
            }
        },
        _ => {}
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

    update_client(app, model);
}

fn update_client(app: &App, model: &mut Model) {
    let client_arc = Arc::clone(&model.client);
    {
        let client = Arc::clone(&client_arc);

        tokio::spawn(async move {
            let mut client = client.lock().await;
            if client.get_token().is_empty() && client.clone().can_recieve(){
                client.generate_token().await;
            }
        });
    }

    if app.time > model.next_refresh {
        let client = Arc::clone(&client_arc);
        let image_sender = model.img_send.clone();
        tokio::spawn(async move {
            let mut client = client.lock().await;
            if !client.get_token().is_empty() && client.clone().can_recieve() {
                client.refresh_track().await;
                let bytes = client.get_image_data().await;
                image_sender.send(bytes).expect("Could not send");
            }
        });
        model.next_refresh +=  2.;
    }

    if let Ok(bytes) =  model.img_recieve.try_recv(){
        if bytes != model.cached_bytes {
            let album_art = nannou::image::load_from_memory(&bytes).expect("Unable to load");
            model.cached_bytes = bytes;
            model.texture = wgpu::Texture::from_image(app, &album_art);
        }
    }
    if model.first_frame {
        let window = app.window(model.window).unwrap();
        window.set_visible(true);
        model.first_frame = false;
    }
}

fn view(app: &App, model: &Model, frame: Frame){  
    app.window(model.window).unwrap().set_cursor_visible(false);
    let draw = app.draw();
    //frame.clear(BLACK);
    draw.background().color(BLACK);
    draw.texture(&model.texture)
    .x_y(model.x, model.y)
    .w_h(SIZE,SIZE);
    draw.to_frame(app, &frame).unwrap();
}

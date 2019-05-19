use super::{AnimationTimer, AudioType, Event, Window, Graphics, Image, Settings, State, Transform};
// use piston_window::keyboard::Key;
// use piston_window::mouse::{MouseButton, MouseCursorEvent};
// use piston_window::{
//     rectangle, Button, Context, EventLoop, EventSettings, Filter, Flip, G2d, Glyphs,
//     Image as GfxImage, ImageSize, PistonWindow, PressEvent, ReleaseEvent, Text, Texture,
//     TextureSettings, Transformed, UpdateEvent, Window as WindowTrait, WindowSettings,
// };

use coffee::graphics::{View, Text, Color, Font, Quad, Rectangle, Point, Frame, Vector, Transformation, Window as CoffeeWindow, Image as CoffeeImage, Target};

use rodio::Source;
use std::any::Any;
use std::cell::RefCell;
use std::fs::File;
use std::io::BufReader;
use std::io::Cursor;
use std::path::Path;
use std::rc::Rc;
use std::thread;
use std::time::Duration;
// use winit::Icon;

struct PcWindow<'a>{
    timer: &'a mut AnimationTimer,
    window: &'a mut CoffeeWindow,
}
impl <'a> Window for PcWindow<'a>{
    fn set_update_rate(&mut self, ups: u64){
        self.timer.set_fps(ups as f64);
    }

    fn load_image_alpha(&mut self, image: image::RgbaImage) -> Result<Image, String>{
        match CoffeeImage::from_image(self.window.gpu(), image::DynamicImage::ImageRgba8(image)){
            Ok(image) => Ok(Image { image }),
            Err(err) => Err(format!("{:?}", err))
        }
    }

    fn load_image(&mut self, path: &str) -> Result<Image, String> {
        let path = "./static/".to_owned() + path;
        match CoffeeImage::new(self.window.gpu(), path){
            Ok(image) => Ok(Image { image }),
            Err(err) => Err(format!("{:?}", err))
        }
    }
}

struct PcGraphics<'a> {
    target: Target<'a>,
    font: &'a mut Font,
}

impl<'a> PcGraphics<'a> {
    fn transform(&self, transform: Option<Transform>) -> Target<'a> {
        if let Some(transform) = transform {
            self.target.transform(Transformation::translate(Vector::new(transform.translate.0 as f32, transform.translate.1 as f32)));
            self.target.transform(Transformation::rotate(transform.rotate as f32));
        }
        self.target
    }
}

impl<'a> Graphics for PcGraphics<'a> {
    fn clear(&mut self, color: &[u8; 4]) {
        self.target.clear(Color::new(color[0] as f32/255.0, color[1] as f32/255.0, color[2] as f32/255.0, color[3] as f32/255.0));
    }

    fn draw_image(
        &mut self,
        transform: Option<Transform>,
        image: &Image,
        src: Option<[f64; 4]>,
        dest: Option<[f64; 4]>,
    ){
        let (w, h) = (image.image.width(), image.image.height());

        let (position, size) = if let Some(dest) = dest{
            (Point::new(dest[0] as f32, dest[1] as f32), (dest[2] as f32, dest[3] as f32))
        }else{
            (Point::new(0.0, 0.0), (w as f32, h as f32))
        };

        let source = if let Some(src) = src{
            Rectangle {
                x: src[0] as f32/w as f32,
                y: src[1] as f32/h as f32,
                width: src[2] as f32/h as f32,
                height: src[3] as f32/h as f32,
            }
        }else{
            Rectangle {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            }
        };

        image.image.draw(
            Quad {
                source,
                position,
                size,
            },
            &mut self.target
        );
    }

    fn draw_text(
        &mut self,
        cotnent: &str,
        x: f64,
        y: f64,
        color: &[u8; 4],
        font_size: u32,
    ){
        self.font.add(Text {
            content: String::from(cotnent),
            position: Point::new(x as f32, y as f32),
            size: font_size as f32,
            color: Color::new(color[0] as f32/255.0, color[1] as f32/255.0, color[2] as f32/255.0, color[3] as f32/255.0),
            ..Text::default()
        });
    }
}

/// 启动线程播放声音
pub fn play_sound(assets: &mut super::AssetsFile, _t: AudioType) {
    if let Some(data) = assets.data() {
        let data = data.to_vec();
        thread::spawn(move || {
            let device = rodio::default_output_device();
            if device.is_none() {
                eprintln!("no default output device.");
                return;
            }
            let sink = rodio::Sink::new(&device.unwrap());
            let decoder = rodio::Decoder::new(Cursor::new(data.to_vec()));
            if decoder.is_err() {
                eprintln!("{:?}", decoder.err());
                return;
            }
            sink.append(decoder.unwrap());
            sink.sleep_until_end();
        });
    }
}

pub fn run<S: State>(title: &str, width: f64, height: f64, settings: Settings) {
    //第一次启动窗口不移动鼠标也会触发一次mouse move事件，过滤这个事件
    let mut got_first_mouse_event = false;
    let size = if let Some(size) = settings.window_size {
        [size.0, size.1]
    } else {
        [width, height]
    };
    let mut window: PistonWindow = WindowSettings::new(title, size)
        .exit_on_esc(true)
        .build()
        .unwrap();
    window.set_event_settings(EventSettings {
        ups: settings.ups,
        ..Default::default()
    });

    let mut state = S::new(&mut BasicWindow{window: &mut window});

    let mut glyphs = None;
    if let Some(font) = settings.font_file {
        let font = "./static/".to_owned() + font;
        match Glyphs::new(
            font.clone(),
            window.factory.clone(),
            TextureSettings::new().filter(Filter::Nearest),
        ) {
            Ok(g) => glyphs = Some(RefCell::new(g)),
            Err(err) => state.handle_error(format!("font load failed! {} {:?}", font, err)),
        };
    }

    if let Some(path) = settings.icon_path {
        let path = "./static/".to_owned() + path;
        let icon = Icon::from_path(path).unwrap();
        window.window.window.set_window_icon(Some(icon));
    }

    let mut mouse_pos = [0.0; 2];

    let background_color = settings.background_color.unwrap_or([0, 0, 0, 255]);
    let draw_center = settings.draw_center;
    let auto_scale = settings.auto_scale;
    let mut scale_x = 1.0;
    let mut scale_y = 1.0;
    let mut trans_x = 0.0;
    let mut trans_y = 0.0;
    while let Some(event) = window.next() {
        let window_size = window.size();
        event.update(|_u| {
            state.update(&mut BasicWindow{window: &mut window});
        });
        window.draw_2d(&event, |context, graphics| {
            //填充背景
            let bgcolor = [
                background_color[0] as f32 / 255.0,
                background_color[1] as f32 / 255.0,
                background_color[2] as f32 / 255.0,
                background_color[3] as f32 / 255.0,
            ];

            let (mut new_width, mut new_height) = (width, height);
            let c = if auto_scale {
                //画面不超过窗口高度
                new_height = window_size.height;
                new_width = new_height / height * width;

                if new_width > window_size.width {
                    new_width = window_size.width;
                    new_height = new_width / width * height;
                }
                scale_x = new_width / width;
                scale_y = new_height / height;
                context.scale(scale_x, scale_y)
            } else {
                scale_x = 1.0;
                scale_y = 1.0;
                context
            };
            let c = if draw_center {
                trans_x = (window_size.width - new_width) / 2.;
                trans_y = (window_size.height - new_height) / 2.;
                c.trans(trans_x / scale_x, trans_y / scale_y)
            } else {
                trans_x = 0.0;
                trans_y = 0.0;
                c
            };
            match state.draw(&mut PcGraphics {
                glyphs: glyphs.as_ref(),
                context: c,
                graphics: graphics,
            }) {
                Ok(()) => (),
                Err(err) => state.handle_error(format!("font load failed! {:?}", err)),
            };
            //遮盖上部分窗口
            rectangle(
                bgcolor,
                [0., 0., window_size.width, trans_y],
                context.transform,
                graphics,
            );
            //遮盖下部分窗口
            rectangle(
                bgcolor,
                [
                    0.,
                    trans_y + height * scale_y,
                    window_size.width,
                    window_size.height - (trans_y + height * scale_y),
                ],
                context.transform,
                graphics,
            );
            //遮盖左部分窗口
            rectangle(
                bgcolor,
                [0.0, 0.0, trans_x, window_size.height],
                context.transform,
                graphics,
            );
            //遮盖右部分窗口
            rectangle(
                bgcolor,
                [
                    trans_x + width * scale_x,
                    0.0,
                    window_size.width - (trans_x + width * scale_x),
                    window_size.height,
                ],
                context.transform,
                graphics,
            );
        });
        event.mouse_cursor(|x, y| {
            if got_first_mouse_event {
                mouse_pos[0] = x;
                mouse_pos[1] = y;
                state.event(Event::MouseMove(
                    (x - trans_x) / scale_x,
                    (y - trans_y) / scale_y,
                ), &mut BasicWindow{window: &mut window});
            } else {
                got_first_mouse_event = true;
            }
        });
        if let Some(Button::Mouse(mouse)) = event.press_args() {
            match mouse {
                MouseButton::Left => state.event(Event::Click(
                    (mouse_pos[0] - trans_x) / scale_x,
                    (mouse_pos[1] - trans_y) / scale_y,
                ), &mut BasicWindow{window: &mut window}),
                _ => (),
            };
        }

        if let Some(Button::Keyboard(key)) = event.release_args() {
            match key {
                Key::D0
                | Key::D1
                | Key::D2
                | Key::D3
                | Key::D4
                | Key::D5
                | Key::D6
                | Key::D7
                | Key::D8
                | Key::D9
                | Key::NumPad0
                | Key::NumPad1
                | Key::NumPad2
                | Key::NumPad3
                | Key::NumPad4
                | Key::NumPad5
                | Key::NumPad6
                | Key::NumPad7
                | Key::NumPad8
                | Key::NumPad9 => {
                    let key = format!("{:?}", key);
                    state.event(Event::KeyUp(key.replace("D", "").replace("NumPad", "")), &mut BasicWindow{window: &mut window})
                }
                Key::LCtrl | Key::RCtrl => {
                    state.event(Event::KeyUp(String::from("CONTROL")), &mut BasicWindow{window: &mut window})
                }
                _ => state.event(Event::KeyUp(format!("{:?}", key)), &mut BasicWindow{window: &mut window}),
            };
        };

        if let Some(Button::Keyboard(key)) = event.press_args() {
            match key {
                Key::D0
                | Key::D1
                | Key::D2
                | Key::D3
                | Key::D4
                | Key::D5
                | Key::D6
                | Key::D7
                | Key::D8
                | Key::D9
                | Key::NumPad0
                | Key::NumPad1
                | Key::NumPad2
                | Key::NumPad3
                | Key::NumPad4
                | Key::NumPad5
                | Key::NumPad6
                | Key::NumPad7
                | Key::NumPad8
                | Key::NumPad9 => {
                    let key = format!("{:?}", key);
                    state.event(Event::KeyDown(key.replace("D", "").replace("NumPad", "")), &mut BasicWindow{window: &mut window})
                }
                Key::LCtrl | Key::RCtrl => {
                    state.event(Event::KeyDown(String::from("CONTROL")), &mut BasicWindow{window: &mut window})
                }
                _ => state.event(Event::KeyDown(format!("{:?}", key)), &mut BasicWindow{window: &mut window}),
            };
        };
    }
}

pub fn current_timestamp() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as f64
}

pub fn random() -> f64 {
    rand::random::<f64>()
}

pub fn log<T: std::fmt::Debug>(s: T) {
    println!("{:?}", s);
}

thread_local! {
    static PLAYING_BACKGROUND: RefCell<bool> = RefCell::new(false);
}

pub fn play_music(file: &str, repeat: bool) {
    let device = rodio::default_output_device().unwrap();
    let path = "./static/".to_owned() + file;
    let file = File::open(path).unwrap();
    let source = rodio::Decoder::new(BufReader::new(file)).unwrap();
    if repeat {
        let stopable = source
            .repeat_infinite()
            .convert_samples()
            .stoppable()
            .periodic_access(Duration::from_millis(100), move |src| {
                if !PLAYING_BACKGROUND.with(|p| *p.borrow()) {
                    src.stop();
                }
            });
        rodio::play_raw(&device, stopable);
    } else {
        rodio::play_raw(&device, source.convert_samples().stoppable());
    }
    PLAYING_BACKGROUND.with(|p| {
        *p.borrow_mut() = true;
    });
}

pub fn stop_music() {
    PLAYING_BACKGROUND.with(|p| {
        *p.borrow_mut() = false;
    });
}

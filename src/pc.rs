use super::{AudioType, Event, Window, Graphics, Image, Settings, State, Transform};
use piston_window::keyboard::Key;
use piston_window::mouse::{MouseButton, MouseCursorEvent};
use piston_window::{
    rectangle, Button, Context, EventLoop, EventSettings, Filter, Flip, G2d, Glyphs,
    Image as GfxImage, ImageSize, PistonWindow, PressEvent, ReleaseEvent, Text, Texture,
    TextureSettings, Transformed, UpdateEvent, Window as WindowTrait, WindowSettings,
};
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
use winit::Icon;

struct PcImage {
    width: f64,
    height: f64,
    texture: gfx_texture::Texture<gfx_device_gl::Resources>,
}
impl Image for PcImage {
    fn width(&self) -> f64 {
        self.width
    }
    fn height(&self) -> f64 {
        self.height
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct BasicWindow<'a>{
    window: &'a mut PistonWindow,
}
impl<'a> Window for BasicWindow<'a> {
    fn set_update_rate(&mut self, ups: u64){
        self.window.set_ups(ups);
    }

    fn load_image_alpha(&mut self, image: &image::RgbaImage) -> Result<Rc<Image>, String>{
        match Texture::from_image(
            &mut self.window.factory,
            image,
            &TextureSettings::new(),
        ){
            Ok(texture) => Ok(Rc::new(PcImage {
                width: texture.get_width() as f64,
                height: texture.get_height() as f64,
                texture,
            })),
            Err(err) => Err(format!("{:?}", err))
        }
    }

    fn load_image(&mut self, path: &str) -> Result<Rc<Image>, String> {
        let path = "./static/".to_owned() + path;
        let texture = Texture::from_path(
            &mut self.window.factory,
            Path::new(&path),
            Flip::None,
            &TextureSettings::new(),
        )?;
        Ok(Rc::new(PcImage {
            width: texture.get_width() as f64,
            height: texture.get_height() as f64,
            texture,
        }))
    }
}

struct PcGraphics<'a, 'b> {
    glyphs: Option<&'a RefCell<Glyphs>>,
    context: Context,
    graphics: &'a mut G2d<'b>,
}

impl<'a, 'b> PcGraphics<'a, 'b> {
    fn transform(&self, transform: Option<Transform>) -> Context {
        let mut context = self.context.clone();
        if let Some(transform) = transform {
            context = context.trans(transform.translate.0, transform.translate.1);
            context = context.rot_rad(transform.rotate);
        }
        context
    }
}

impl<'a, 'b> Graphics for PcGraphics<'a, 'b> {
    fn clear_rect(&mut self, color: &[u8; 4], x: f64, y: f64, width: f64, height: f64) {
        rectangle(
            [
                color[0] as f32 / 255.0,
                color[1] as f32 / 255.0,
                color[2] as f32 / 255.0,
                color[3] as f32 / 255.0,
            ], // red
            [x, y, width, height],
            self.context.transform,
            self.graphics,
        );
    }

    fn draw_image(
        &mut self,
        transform: Option<Transform>,
        image: &Image,
        src: Option<[f64; 4]>,
        dest: Option<[f64; 4]>,
    ) -> Result<(), String> {
        match image.as_any().downcast_ref::<PcImage>() {
            Some(image) => {
                let context = self.transform(transform);
                let mut gfx_image = GfxImage::new();
                if let Some(src) = src {
                    gfx_image = gfx_image.src_rect(src);
                }
                if let Some(dest) = dest {
                    gfx_image = gfx_image.rect(dest);
                }
                gfx_image.draw(
                    &image.texture,
                    &context.draw_state,
                    context.transform,
                    self.graphics,
                );
                Ok(())
            }
            None => Err("Image downcast PcImage Error!".to_string()),
        }
    }

    fn draw_text(
        &mut self,
        transform: Option<Transform>,
        cotnent: &str,
        x: f64,
        y: f64,
        color: &[u8; 4],
        font_size: u32,
    ) -> Result<(), String> {
        let context = self.transform(transform);
        if let Some(glyphs) = self.glyphs.as_mut() {
            let text = Text::new_color(
                [
                    color[0] as f32 / 255.0,
                    color[1] as f32 / 255.0,
                    color[2] as f32 / 255.0,
                    color[3] as f32 / 255.0,
                ],
                font_size,
            );
            match text.draw(
                cotnent,
                &mut *glyphs.borrow_mut(),
                &context.draw_state,
                context.trans(x, y).transform,
                self.graphics,
            ) {
                Err(err) => Err(format!("{:?}", err)),
                Ok(()) => Ok(()),
            }
        } else {
            Ok(())
        }
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

use super::{Assets, AssetsType, AudioType, Event, Graphics, Settings, State, Transform, Window};
use direct2d::brush::SolidColorBrush;
use direct2d::enums::{
    BitmapInterpolationMode, DrawTextOptions, PresentOptions, RenderTargetType, RenderTargetUsage,
};
use direct2d::image::Bitmap;
use direct2d::render_target::hwnd::HwndRenderTarget;
use directwrite::text_format::TextFormat;
use dxgi::enums::*;
use image::RgbaImage;
use math2d::*;
use rodio::Source;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::io::{BufReader, Cursor, Error, ErrorKind, Result};
use std::rc::Rc;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use winapi::shared::windef::HWND;
use winit::dpi::LogicalSize;
use winit::{ElementState, MouseButton, VirtualKeyCode};

#[derive(Debug, Clone)]
pub struct Sound {
    audio_type: AudioType,
    buffer: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Image {
    bitmap: Rc<Bitmap>,
    width: f64,
    height: f64,
}

impl Image {
    pub fn width(&self) -> f64 {
        self.width
    }
    pub fn height(&self) -> f64 {
        self.height
    }
}

enum RawAssets {
    Image(RgbaImage),
    Blob(Vec<u8>),
}

struct D2DWindow {
    new_size: Option<(f64, f64)>,
    thread_sender: Sender<(String, AssetsType, Result<RawAssets>)>,
    update_delay: Duration,
}
impl Window for D2DWindow {
    fn set_update_rate(&mut self, ups: u64) {
        self.update_delay = Duration::from_micros(1000 * 1000 / ups);
    }

    fn load_assets(&mut self, assets: Vec<(&str, AssetsType)>) {
        let sender = self.thread_sender.clone();
        let assets: Vec<(String, AssetsType)> = assets
            .iter()
            .map(|(path, tp)| (path.to_string(), *tp))
            .collect();
        //启动线程读取所有文件
        thread::spawn(move || {
            for (path, tp) in assets {
                let key_path = path;
                let path = "./static/".to_owned() + &key_path;
                match File::open(&path) {
                    Ok(mut f) => {
                        let mut buf = vec![];
                        match f.read_to_end(&mut buf) {
                            Ok(_len) => {
                                match tp {
                                    AssetsType::Image => {
                                        if let Ok(image) = image::load_from_memory(&buf) {
                                            let mut rgba_image = image.to_rgba();
                                            //将白色透明转换为黑色透明
                                            for pixel in rgba_image.chunks_mut(4) {
                                                if pixel[3] == 0 {
                                                    pixel.copy_from_slice(&[0, 0, 0, 0])
                                                }
                                            }
                                            let _ = sender.send((
                                                key_path,
                                                tp,
                                                Ok(RawAssets::Image(rgba_image)),
                                            ));
                                        } else {
                                            let _ = sender.send((
                                                key_path,
                                                tp,
                                                Err(Error::new(
                                                    ErrorKind::Other,
                                                    "图片读取失败",
                                                )),
                                            ));
                                        }
                                    }
                                    _ => {
                                        //将文件数据发送到主线程
                                        let _ =
                                            sender.send((key_path, tp, Ok(RawAssets::Blob(buf))));
                                    }
                                }
                            }
                            Err(err) => {
                                let _ = sender.send((key_path, tp, Err(err)));
                            }
                        }
                    }
                    Err(err) => {
                        let _ = sender.send((key_path, tp, Err(err)));
                    }
                }
            }
        });
    }

    fn load_image(&mut self, width: u32, height: u32, key: &str, data: Vec<u8>) {
        let image: RgbaImage = image::ImageBuffer::from_raw(width, height, data).unwrap();
        let _ = self.thread_sender.send((
            String::from(key),
            AssetsType::Image,
            Ok(RawAssets::Image(image)),
        ));
    }

    fn load_svg(&mut self, key: &str, svg: String) {
        let svg = nsvg::parse_str(&svg, nsvg::Units::Pixel, 96.0).unwrap();
        // Rasterize the loaded SVG and return dimensions and a RGBA buffer
        let (width, height, raw_rgba) = svg.rasterize_to_raw_rgba(1.0).unwrap();
        let mut image = image::DynamicImage::new_rgba8(width, height).to_rgba();
        image.copy_from_slice(&raw_rgba);
        let _ = self.thread_sender.send((
            String::from(key),
            AssetsType::Image,
            Ok(RawAssets::Image(image)),
        ));
    }
}

struct D2DGraphics {
    target: HwndRenderTarget,
    text_formats: HashMap<u32, TextFormat>,
    solid_bursh: HashMap<[u8; 4], SolidColorBrush>,
    dwfactory: directwrite::factory::Factory,
}

impl D2DGraphics {
    fn check_brush(&mut self, color: &[u8; 4]) {
        if !self.solid_bursh.contains_key(color) {
            self.solid_bursh.insert(
                *color,
                SolidColorBrush::create(&self.target)
                    .with_color(Color::new(
                        color[0] as f32 / 255.0,
                        color[1] as f32 / 255.0,
                        color[2] as f32 / 255.0,
                        color[3] as f32 / 255.0,
                    ))
                    .build()
                    .unwrap(),
            );
        }
    }

    fn check_text_format(&mut self, font_size: &u32) {
        if !self.text_formats.contains_key(&font_size) {
            let text_format = TextFormat::create(&self.dwfactory)
                .with_family("")
                .with_size(*font_size as f32)
                .build()
                .unwrap();
            self.text_formats.insert(*font_size, text_format);
        }
    }
}

impl Graphics for D2DGraphics {
    fn fill_rect(&mut self, color: &[u8; 4], x: f64, y: f64, width: f64, height: f64) {
        self.check_brush(color);
        self.target.fill_rectangle(
            [x as f32, y as f32, (x + width) as f32, (y + height) as f32],
            self.solid_bursh.get(color).unwrap(),
        );
    }

    fn draw_image(
        &mut self,
        transform: Option<Transform>,
        image: &Image,
        src: Option<[f64; 4]>,
        dest: Option<[f64; 4]>,
    ) {
        let (w, h) = (image.width(), image.height());

        let dest_rect = if let Some(dest) = dest {
            Rectf::new(
                dest[0] as f32,
                dest[1] as f32,
                (dest[0] + dest[2]) as f32,
                (dest[3] + dest[1]) as f32,
            )
        } else {
            Rectf::new(0.0, 0.0, w as f32, h as f32)
        };

        let src_rect = if let Some(src) = src {
            Rectf::new(
                src[0] as f32,
                src[1] as f32,
                (src[0] + src[2]) as f32,
                (src[1] + src[3]) as f32,
            )
        } else {
            Rectf::new(0.0, 0.0, w as f32, h as f32)
        };

        let old_transform = self.target.transform();
        let t = if let Some(transform) = transform {
            Matrix3x2f::rotation(transform.rotate as f32, [0.5, 0.5])
            *
            //translation要先乘以之前的变比
            old_transform * Matrix3x2f::translation([
                transform.translate.0 as f32 * old_transform.a,
                transform.translate.1 as f32 * old_transform.d,
            ])
        } else {
            old_transform
        };
        self.target.set_transform(&t);
        self.target.draw_bitmap(
            &image.bitmap,
            dest_rect,
            1.0,
            BitmapInterpolationMode::Linear,
            src_rect,
        );
        self.target.set_transform(&old_transform);
    }

    fn draw_text(&mut self, cotnent: &str, x: f64, y: f64, color: &[u8; 4], font_size: u32) {
        self.check_brush(color);
        self.check_text_format(&font_size);
        self.target.draw_text(
            cotnent,
            &self.text_formats.get(&font_size).unwrap(),
            [
                x as f32,
                y as f32,
                x as f32 + (font_size as f32 * cotnent.len() as f32),
                y as f32 + (font_size as f32),
            ],
            self.solid_bursh.get(color).unwrap(),
            DrawTextOptions::NONE,
        );
    }
}

/// 启动线程播放声音
pub fn play_sound(sound: &Sound) {
    let buffer = sound.buffer.clone();
    thread::spawn(move || {
        let device = rodio::default_output_device();
        if device.is_none() {
            eprintln!("no default output device.");
            return;
        }
        let sink = rodio::Sink::new(&device.unwrap());
        let decoder = rodio::Decoder::new(Cursor::new(buffer));
        if decoder.is_err() {
            eprintln!("{:?}", decoder.err());
            return;
        }
        sink.append(decoder.unwrap());
        sink.sleep_until_end();
    });
}

pub fn run<S: State>(title: &str, width: f64, height: f64, settings: Settings) {
    //第一次启动窗口不移动鼠标也会触发一次mouse move事件，过滤这个事件
    let initial_window_size = if let Some(size) = settings.window_size {
        [size.0, size.1]
    } else {
        [width, height]
    };

    let mut events_loop = winit::EventsLoop::new();

    let window = winit::WindowBuilder::new()
        .with_dimensions(LogicalSize::new(
            initial_window_size[0],
            initial_window_size[1],
        ))
        .with_title(title)
        .build(&events_loop)
        .unwrap();

    if let Some(path) = settings.icon_path {
        let path = "./static/".to_owned() + path;
        if let Ok(icon) = winit::Icon::from_path(path) {
            window.set_window_icon(Some(icon));
        }
    }

    use winit::os::windows::WindowExt;
    let hwnd = window.get_hwnd() as HWND;

    let d2d = direct2d::factory::Factory::new().unwrap();
    use direct2d::render_target::hwnd::HwndRenderTargetBuilder;
    let target = HwndRenderTargetBuilder::new(&d2d)
        .with_hwnd(hwnd)
        .with_usage(RenderTargetUsage::NONE)
        .with_target_type(RenderTargetType::Default)
        .with_pixel_size(initial_window_size[0] as u32, initial_window_size[1] as u32)
        // .with_format(Format::R8G8B8A8Uint)
        // .with_alpha_mode(direct2d::enums::AlphaMode::Ignore)
        .with_present_options(PresentOptions::NONE)
        // .with_dpi(20.0, 20.0)
        .build()
        .unwrap();

    let graphics = D2DGraphics {
        solid_bursh: HashMap::new(),
        text_formats: HashMap::new(),
        target,
        dwfactory: directwrite::factory::Factory::new().unwrap(),
    };

    let background_color = settings.background_color.unwrap_or([0, 0, 0, 255]);
    //填充背景
    let bg_brush = SolidColorBrush::create(&graphics.target)
        .with_color(Color::new(
            background_color[0] as f32,
            background_color[1] as f32,
            background_color[2] as f32,
            background_color[3] as f32,
        ))
        .build()
        .unwrap();

    //绘制帧率
    let green_brush = SolidColorBrush::create(&graphics.target)
        .with_color(Color::new(1.0, 1.0, 0.0, 1.0))
        .build()
        .unwrap();
    let fps_text_format = TextFormat::create(&graphics.dwfactory)
        .with_family("")
        .with_size(10.0)
        .build()
        .unwrap();

    let graphics = Arc::new(Mutex::new(graphics));

    let mut mouse_pos = [0.0; 2];

    let draw_center = settings.draw_center;
    let auto_scale = settings.auto_scale;
    let mut scale_x = 1.0;
    let mut scale_y = 1.0;
    let mut trans_x = 0.0;
    let mut trans_y = 0.0;

    let (thread_sender, main_receiver) = channel();
    let (main_sender, thread_receiver) = channel();
    let (target_sender, target_receiver) = channel();
    let (assets_sender, assets_receiver) = channel();

    let g = graphics.clone();
    thread::spawn(move || {
        //接收到draw消息，调用begin_draw，然后等待end_draw消息
        let begin_draw = |new_size: Option<Sizeu>| {
            if let Ok(mut g) = g.lock() {
                if let Some(size) = new_size {
                    let _ = g.target.resize(size);
                }
                g.target.begin_draw();
            }
        };

        loop {
            let mut new_size = None;
            if let Ok((msg, data)) = target_receiver.try_recv() {
                if msg == "resize" {
                    new_size = Some(data);
                }
                if msg == "exit" {
                    let _ = thread_sender.send("exit");
                    break;
                }
            }
            begin_draw(new_size);
            let _ = thread_sender.send("begin");
            if let Ok(msg) = thread_receiver.recv() {
                if msg == "draw" {
                    let _ = g.lock().unwrap().target.end_draw();
                }
            }
        }
    });

    let mut raw_rgba_images = vec![];
    let mut game_window = D2DWindow {
        new_size: None,
        update_delay: Duration::from_micros(1000 * 1000 / settings.ups),
        thread_sender: assets_sender,
    };
    let mut game = S::new(&mut game_window);
    let update_timer = Instant::now();
    let mut next_update_time = update_timer.elapsed();
    let mut next_log_time = update_timer.elapsed();
    let next_log_delay = Duration::from_millis(1000);
    let (mut fps, mut ups, mut ups_count, mut fps_count) = (0, 0, 0, 0);
    loop {
        if update_timer.elapsed() >= next_update_time {
            next_update_time = next_update_time + game_window.update_delay;
            game.update(&mut game_window);
            ups_count += 1;
        }
        if let Ok((path, tp, data)) = assets_receiver.try_recv() {
            match data {
                Ok(RawAssets::Blob(data)) => match tp {
                    AssetsType::Sound => game.on_assets_load(
                        &path,
                        tp,
                        Ok(Assets::Sound(Sound {
                            buffer: data,
                            audio_type: AudioType::test(&path),
                        })),
                        &mut game_window,
                    ),
                    _ => game.on_assets_load(&path, tp, Ok(Assets::File(data)), &mut game_window),
                },
                Ok(RawAssets::Image(image)) => {
                    raw_rgba_images.push((path, image));
                }
                Err(err) => game.on_assets_load(&path, tp, Err(err), &mut game_window),
            };
        }

        if let Ok(msg) = main_receiver.try_recv() {
            if msg == "exit" {
                break;
            }
            fps_count += 1;
            let mut g = graphics.lock().unwrap();
            let graphics_size = g.target.size();
            if game_window.new_size.is_some() {
                let new_size = game_window.new_size.take().unwrap();
                let _ = target_sender
                    .send(("resize", Sizeu::new(new_size.0 as u32, new_size.1 as u32)));
            }
            //加载图片资源
            if let Some((path, data)) = raw_rgba_images.pop() {
                let (w, h) = (data.width(), data.height());
                let buf = data.into_raw();
                match Bitmap::create(&g.target)
                    .with_format(Format::R8G8B8A8Unorm)
                    .with_raw_data(Sizeu::new(w, h), &buf, w * 4)
                    .build()
                {
                    Ok(bmp) => {
                        let sz = bmp.size();
                        game.on_assets_load(
                            &path,
                            AssetsType::Image,
                            Ok(Assets::Image(Image {
                                bitmap: Rc::new(bmp),
                                width: sz.width as f64,
                                height: sz.height as f64,
                            })),
                            &mut game_window,
                        )
                    }
                    Err(err) => game.on_assets_load(
                        &path,
                        AssetsType::Image,
                        Err(Error::new(ErrorKind::Other, format!("{:?}", err))),
                        &mut game_window,
                    ),
                };
            }

            let mut transform = Matrix3x2f::IDENTITY;
            let (mut new_width, mut new_height) = (width, height);
            if auto_scale {
                //画面不超过窗口高度
                new_height = graphics_size.height as f64;
                new_width = (new_height / height) as f64 * width as f64;

                if new_width > graphics_size.width as f64 {
                    new_width = graphics_size.width as f64;
                    new_height = (new_width / width) * height;
                }
                scale_x = new_width / width;
                scale_y = new_height / height;
                // println!("scalex={},scaley={}", scale_x, scale_y);
                transform =
                    transform * Matrix3x2f::scaling([scale_x as f32, scale_y as f32], [0.0, 0.0]);
            } else {
                scale_x = 1.0;
                scale_y = 1.0;
            }
            if draw_center {
                trans_x = (graphics_size.width as f64 - new_width) / 2.;
                trans_y = (graphics_size.height as f64 - new_height) / 2.;
                // println!("trans_x={},trans_y={}", trans_x, trans_y);
                transform = transform * Matrix3x2f::translation([trans_x as f32, trans_y as f32]);
            // transform = transform * Matrix3x2f::translation([(trans_x / scale_x) as f32, (trans_y / scale_y) as f32]);
            } else {
                trans_x = 0.0;
                trans_y = 0.0;
            }
            g.target.set_transform(&transform);
            let _ = game.draw(&mut *g, &mut game_window);

            //显示UPS/FPS
            if settings.show_ups_fps {
                g.target.draw_text(
                    &format!("UPS/FPS:{}/{}", ups, fps),
                    &fps_text_format,
                    [20.0, height as f32 - 30., 300.0, height as f32],
                    &green_brush,
                    DrawTextOptions::NONE,
                );
            }
            g.target.set_transform(&Matrix3x2f::IDENTITY);

            //遮盖上部分窗口
            g.target.fill_rectangle(
                [0., 0., graphics_size.width as f32, trans_y as f32],
                &bg_brush,
            );
            //遮盖下部分窗口
            let (x, y, width, height) = (
                0.,
                (trans_y + height * scale_y) as f32,
                graphics_size.width,
                graphics_size.height - (trans_y + height * scale_y) as f32,
            );
            g.target
                .fill_rectangle([x, y, x + width, y + height], &bg_brush);
            //遮盖左部分窗口
            g.target
                .fill_rectangle([0.0, 0.0, trans_x as f32, graphics_size.height], &bg_brush);
            //遮盖右部分窗口
            let (x, y, width, height) = (
                (trans_x + new_width) as f32,
                0.0,
                graphics_size.width - (trans_x + new_width) as f32,
                graphics_size.height,
            );
            g.target
                .fill_rectangle([x, y, x + width, y + height], &bg_brush);

            let _ = main_sender.send("draw");
        }
        if update_timer.elapsed() > next_log_time {
            next_log_time = next_log_time + next_log_delay;
            ups = ups_count;
            fps = fps_count;
            ups_count = 0;
            fps_count = 0;
        }
        events_loop.poll_events(|event| {
            match event {
                winit::Event::WindowEvent {
                    event: winit::WindowEvent::CursorMoved { position, .. },
                    ..
                } => {
                    mouse_pos[0] = position.x;
                    mouse_pos[1] = position.y;
                    game.event(
                        Event::MouseMove(
                            (position.x - trans_x) / scale_x,
                            (position.y - trans_y) / scale_y,
                        ),
                        &mut game_window,
                    );
                    winit::ControlFlow::Continue
                }
                winit::Event::WindowEvent {
                    event: winit::WindowEvent::MouseInput { state, button, .. },
                    ..
                } => {
                    match button {
                        MouseButton::Left => {
                            match state {
                                ElementState::Released => game.event(
                                    Event::Click(
                                        (mouse_pos[0] - trans_x) / scale_x,
                                        (mouse_pos[1] - trans_y) / scale_y,
                                    ),
                                    &mut game_window,
                                ),
                                _ => (),
                            };
                        }
                        _ => (),
                    };
                    winit::ControlFlow::Continue
                }
                winit::Event::DeviceEvent {
                    event: winit::DeviceEvent::Key(input),
                    ..
                } => {
                    if let Some(vk) = input.virtual_keycode {
                        match vk {
                            VirtualKeyCode::Key0
                            | VirtualKeyCode::Key1
                            | VirtualKeyCode::Key2
                            | VirtualKeyCode::Key3
                            | VirtualKeyCode::Key4
                            | VirtualKeyCode::Key5
                            | VirtualKeyCode::Key6
                            | VirtualKeyCode::Key7
                            | VirtualKeyCode::Key8
                            | VirtualKeyCode::Key9
                            | VirtualKeyCode::Numpad0
                            | VirtualKeyCode::Numpad1
                            | VirtualKeyCode::Numpad2
                            | VirtualKeyCode::Numpad3
                            | VirtualKeyCode::Numpad4
                            | VirtualKeyCode::Numpad5
                            | VirtualKeyCode::Numpad6
                            | VirtualKeyCode::Numpad7
                            | VirtualKeyCode::Numpad8
                            | VirtualKeyCode::Numpad9 => {
                                let key =
                                    format!("{:?}", vk).replace("Key", "").replace("Numpad", "");
                                match input.state {
                                    ElementState::Pressed => {
                                        game.event(Event::KeyDown(key), &mut game_window)
                                    }
                                    ElementState::Released => {
                                        game.event(Event::KeyUp(key), &mut game_window)
                                    }
                                };
                            }
                            VirtualKeyCode::LControl | VirtualKeyCode::RControl => {
                                match input.state {
                                    ElementState::Pressed => game.event(
                                        Event::KeyDown(String::from("CONTROL")),
                                        &mut game_window,
                                    ),
                                    ElementState::Released => game.event(
                                        Event::KeyUp(String::from("CONTROL")),
                                        &mut game_window,
                                    ),
                                };
                            }
                            _ => {
                                match input.state {
                                    ElementState::Pressed => game.event(
                                        Event::KeyDown(format!("{:?}", vk)),
                                        &mut game_window,
                                    ),
                                    ElementState::Released => game
                                        .event(Event::KeyUp(format!("{:?}", vk)), &mut game_window),
                                };
                            }
                        };
                    }
                    winit::ControlFlow::Continue
                }
                winit::Event::WindowEvent {
                    event: winit::WindowEvent::Resized(size),
                    ..
                } => {
                    game_window.new_size = Some((size.width, size.height));
                    winit::ControlFlow::Continue
                }
                winit::Event::WindowEvent {
                    event: winit::WindowEvent::CloseRequested,
                    ..
                } => {
                    let _ = target_sender.send(("exit", Sizeu::new(0, 0)));
                    winit::ControlFlow::Break
                }
                _ => winit::ControlFlow::Continue,
            };
        });

        if game_window.update_delay > Duration::from_millis(10) {
            thread::sleep(Duration::from_nanos(1));
        }
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

pub fn alert(head: &str, msg: &str) {
    // fn alert_message(head: &str,msg: &str,_wflags: i32 ) -> Result<i32, Error> {
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;
    use winapi::um::winuser::{MessageBoxW, MB_OK};
    let whead: Vec<u16> = OsStr::new(head).encode_wide().chain(once(0)).collect();
    let wmsg: Vec<u16> = OsStr::new(msg).encode_wide().chain(once(0)).collect();
    let _ret = unsafe {
        MessageBoxW(
            null_mut(),
            wmsg.as_ptr(),
            whead.as_ptr(),
            MB_OK | winapi::um::winuser::MB_ICONEXCLAMATION,
        )
    };
    // if ret == 0 {
    //     Err(Error::last_os_error())
    // }else {
    //     Ok(ret)
    // }
    // }
}

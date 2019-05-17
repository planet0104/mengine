#![recursion_limit = "256"]

#[cfg(any(target_arch = "asmjs", target_arch = "wasm32"))]
#[macro_use]
extern crate stdweb;

use std::any::Any;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[cfg(not(any(target_arch = "asmjs", target_arch = "wasm32")))]
mod pc;
#[cfg(not(any(target_arch = "asmjs", target_arch = "wasm32")))]
use pc as window;

#[cfg(any(target_arch = "asmjs", target_arch = "wasm32"))]
mod web;
#[cfg(any(target_arch = "asmjs", target_arch = "wasm32"))]
use web as window;

pub mod engine;

pub use window::{current_timestamp, log, play_music, play_sound, random, run, stop_music};

pub struct Transform {
    pub rotate: f64,
    pub translate: (f64, f64),
}
impl Default for Transform {
    fn default() -> Self {
        Transform {
            rotate: 0.0,
            translate: (0.0, 0.0),
        }
    }
}

pub trait Window {
    fn set_update_rate(&mut self, ups: u64);
    fn load_image(&mut self, path: &str) -> Result<Rc<Image>, String>;
    fn load_image_alpha(&mut self, image: &image::RgbaImage) -> Result<Rc<Image>, String>;
}

pub trait Graphics {
    fn clear_rect(&mut self, color: &[u8; 4], x: f64, y: f64, width: f64, height: f64);
    /// 绘制图片
    ///
    /// # Arguments
    ///
    /// * `image` Image
    /// * `src` Option<[x, y, w, h]>
    /// * `dest` Option<[x, y, w, h]>
    ///
    /// # Example
    ///
    /// ```
    /// g.draw_image(&image, Some([0, 0, 100, 100]), Some([0, 0, 100, 100])).expect("error!");
    /// ```
    fn draw_image(
        &mut self,
        transform: Option<Transform>,
        image: &Image,
        src: Option<[f64; 4]>,
        dest: Option<[f64; 4]>,
    ) -> Result<(), String>;

    fn draw_image_at(
        &mut self,
        transform: Option<Transform>,
        image: &Image,
        x: f64,
        y: f64,
    ) -> Result<(), String> {
        self.draw_image(
            transform,
            image,
            None,
            Some([x, y, image.width(), image.height()]),
        )
    }

    /// 绘制文字
    ///
    /// # Arguments
    ///
    /// * `content`
    /// * `x`
    /// * `y`
    /// * `color` &[u8; 4]
    /// * `font_size` 字体 单位pt
    /// # Example
    ///
    /// ```
    /// g.draw_text("Hello!", 0., 20., &[255, 0, 0, 255], 16).expect("text draw failed.");
    /// ```
    fn draw_text(
        &mut self,
        transform: Option<Transform>,
        cotnent: &str,
        x: f64,
        y: f64,
        color: &[u8; 4],
        font_size: u32,
    ) -> Result<(), String>;
}

#[derive(Debug)]
pub enum Event {
    MouseMove(f64, f64),
    Click(f64, f64),
    KeyDown(String),
    KeyUp(String),
}

pub trait State: 'static {
    fn new(window: &mut Window) -> Self;
    fn update(&mut self, _window: &mut Window) {}
    fn event(&mut self, _event: Event, _window: &mut Window) {}
    fn draw(&mut self, _graphics: &mut Graphics) -> Result<(), String> {
        Ok(())
    }
    #[cfg(any(target_arch = "asmjs", target_arch = "wasm32"))]
    fn handle_error(&mut self, error: String) {
        console!(error, error);
    }
    #[cfg(not(any(target_arch = "asmjs", target_arch = "wasm32")))]
    fn handle_error(&mut self, error: String) {
        eprintln!("Unhandled error: {:?}", error);
    }
}

pub trait Image {
    fn width(&self) -> f64;
    fn height(&self) -> f64;
    fn as_any(&self) -> &dyn Any;
}

//计时器
#[derive(Clone)]
pub struct AnimationTimer {
    frame_time: f64,
    next_time: f64,
}

impl AnimationTimer {
    pub fn new(fps: f64) -> AnimationTimer {
        AnimationTimer {
            frame_time: 1000.0 / fps,
            next_time: current_timestamp(),
        }
    }

    pub fn set_fps(&mut self, fps: f64){
        self.frame_time = 1000.0 / fps;
    }

    pub fn reset(&mut self) {
        self.next_time = current_timestamp();
    }

    pub fn ready_for_next_frame(&mut self) -> bool {
        let now = current_timestamp();
        if now >= self.next_time {
            //更新时间
            self.next_time += self.frame_time;
            true
        } else {
            false
        }
    }
}

pub struct SubImage {
    image: Rc<Image>,
    region: [f64; 4],
}

impl SubImage {
    pub fn new(image: Rc<Image>, region: [f64; 4]) -> SubImage {
        SubImage { image, region }
    }

    pub fn draw(
        &self,
        transform: Option<Transform>,
        g: &mut Graphics,
        dest: [f64; 4],
    ) -> Result<(), String> {
        g.draw_image(
            transform,
            self.image.as_ref(),
            Some(self.region),
            Some(dest),
        )
    }
}

#[derive(Clone)]
pub struct Animation {
    timer: AnimationTimer,
    image: Rc<Image>,
    frames: Vec<[f64; 4]>,
    current: i32,
    repeat: bool,
    active: bool,
    end: bool, //current == frames.len()
    pub position: Option<[f64; 4]>,
}

impl Animation {
    pub fn new(image: Rc<Image>, frames: Vec<[f64; 4]>, fps: f64) -> Animation {
        Animation {
            timer: AnimationTimer::new(fps),
            image,
            frames,
            current: -1,
            repeat: false,
            active: false,
            end: false,
            position: None,
        }
    }

    pub fn active(image: Rc<Image>, frames: Vec<[f64; 4]>, fps: f64) -> Animation {
        let mut anim = Self::new(image, frames, fps);
        anim.start();
        anim
    }

    pub fn frame_width(&self) -> f64 {
        if self.frames.len() == 0 {
            0.0
        } else {
            self.frames[0][2]
        }
    }

    pub fn frame_height(&self) -> f64 {
        if self.frames.len() == 0 {
            0.0
        } else {
            self.frames[0][3]
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn set_repeat(&mut self, repeat: bool) {
        self.repeat = repeat;
    }

    pub fn is_repeat(&mut self) -> bool {
        self.repeat
    }

    pub fn start(&mut self) {
        self.active = true;
        self.current = -1;
        self.timer.reset();
    }

    pub fn stop(&mut self) {
        self.active = false;
    }

    pub fn is_end(&self) -> bool {
        self.current == self.frames.len() as i32
    }

    /// Tick the animation forward by one step
    pub fn update(&mut self) -> bool {
        let mut jump = false;
        if self.active {
            if self.timer.ready_for_next_frame() {
                self.current += 1;
                if self.current == self.frames.len() as i32 {
                    if self.repeat {
                        self.current = 0;
                    } else {
                        self.active = false;
                    }
                }
                jump = true;
            }
        }
        jump
    }

    pub fn draw(
        &self,
        transform: Option<Transform>,
        g: &mut Graphics,
        dest: [f64; 4],
    ) -> Result<(), String> {
        let mut current = 0;
        if self.current > 0 {
            current = if self.current == self.frames.len() as i32 {
                self.frames.len() as i32 - 1
            } else {
                self.current
            };
        }
        g.draw_image(
            transform,
            self.image.as_ref(),
            Some(self.frames[current as usize]),
            Some(dest),
        )
    }
}

#[derive(Clone, Copy)]
pub struct Rect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl Rect {
    pub fn new(left: f64, top: f64, right: f64, bottom: f64) -> Rect {
        Rect {
            left: left,
            top: top,
            right: right,
            bottom: bottom,
        }
    }

    pub fn zero() -> Rect {
        Rect {
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
        }
    }

    /** 修改rect大小 */
    pub fn inflate(&mut self, dx: f64, dy: f64) {
        self.left -= dx;
        self.right += dx;
        self.top -= dy;
        self.bottom += dy;
    }

    pub fn offset(&mut self, dx: f64, dy: f64) {
        self.left += dx;
        self.right += dx;
        self.top += dy;
        self.bottom += dy;
    }

    pub fn contain(&self, x: f64, y: f64) -> bool {
        x >= self.left && x <= self.right && y >= self.top && y <= self.bottom
    }
}

#[derive(Clone, Copy)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

// use std::cmp::PartialOrd;
// use std::ops::{Add, AddAssign, Sub, SubAssign};

// #[derive(Clone, Debug)]
// pub struct Rect<T: PartialOrd + Add + Sub + AddAssign + SubAssign + Copy + Default> {
//     pub pos: Point<T>,
//     pub size: Size<T>,
// }

// impl<
//         T: PartialOrd + Add<Output = T> + Sub<Output = T> + AddAssign + SubAssign + Copy + Default,
//     > Default for Rect<T>
// {
//     fn default() -> Self {
//         Rect {
//             pos: Point::default(),
//             size: Size::default(),
//         }
//     }
// }

// impl<
//         T: PartialOrd + Add<Output = T> + Sub<Output = T> + AddAssign + SubAssign + Copy + Default,
//     > Rect<T>
// {
//     pub fn new(x: T, y: T, width: T, height: T) -> Rect<T> {
//         Rect {
//             pos: Point::new(x, y),
//             size: Size::new(width, height),
//         }
//     }

//     pub fn left(&self) -> T {
//         self.pos.x
//     }

//     pub fn top(&self) -> T {
//         self.pos.y
//     }

//     pub fn right(&self) -> T {
//         self.pos.x + self.size.width
//     }

//     pub fn bottom(&self) -> T {
//         self.pos.y + self.size.height
//     }

//     pub fn width(&self) -> T {
//         self.size.width
//     }

//     pub fn height(&self) -> T {
//         self.size.height
//     }

//     pub fn inflate(&mut self, dx: T, dy: T) {
//         self.pos.x -= dx;
//         self.size.width += dx + dx;
//         self.pos.y -= dy;
//         self.size.height += dy + dy;
//     }

//     pub fn offset(&mut self, dx: T, dy: T) {
//         self.pos.x -= dx;
//         self.pos.y -= dy;
//     }

//     pub fn move_to(&mut self, x: T, y: T) {
//         self.pos.x = x;
//         self.pos.y = y;
//     }

//     pub fn contain(&self, x: T, y: T) -> bool {
//         x >= self.pos.x && x <= self.right() && y >= self.pos.y && y <= self.bottom()
//     }

//     pub fn to_slice(&self) -> [T; 4] {
//         [self.pos.x, self.pos.y, self.size.width, self.size.height]
//     }
// }

// #[derive(Clone, Debug, Copy)]
// pub struct Point<T: Default> {
//     pub x: T,
//     pub y: T,
// }

// impl<T: Default> Point<T> {
//     pub fn new(x: T, y: T) -> Point<T> {
//         Point { x, y }
//     }
// }

// impl<T: Default> Default for Point<T> {
//     fn default() -> Self {
//         Point {
//             x: T::default(),
//             y: T::default(),
//         }
//     }
// }

#[derive(Clone, Debug, Copy)]
pub struct Size<T: Default> {
    pub width: T,
    pub height: T,
}

impl<T: Default> Default for Size<T> {
    fn default() -> Self {
        Size {
            width: T::default(),
            height: T::default(),
        }
    }
}

impl<T: Default> Size<T> {
    pub fn new(width: T, height: T) -> Size<T> {
        Size { width, height }
    }
}

///A builder that constructs a Window
#[derive(Debug)]
pub struct Settings {
    /// If the cursor should be visible over the application
    pub show_cursor: bool,
    /// The smallest size the user can resize the window to
    ///
    /// Does nothing on web
    pub min_size: Option<Size<f64>>,
    /// The largest size the user can resize the window to
    ///
    /// Does nothing on web
    pub max_size: Option<Size<f64>>,
    pub fullscreen: bool,
    /// How many times is the update method called per second
    pub ups: u64,
    pub icon_path: Option<&'static str>, // TODO: statiC?
    /// 字体文件名(static文件夹)
    pub font_file: Option<&'static str>,
    /// 背景色[r,g,b,a]
    pub background_color: Option<[u8; 4]>,
    pub window_size: Option<(f64, f64)>,
    /// 居中绘图
    pub draw_center: bool,
    // 自动缩放
    pub auto_scale: bool,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            show_cursor: true,
            min_size: None,
            max_size: None,
            fullscreen: false,
            ups: 60,
            icon_path: None,
            font_file: None,
            background_color: None,
            draw_center: true,
            auto_scale: false,
            window_size: None,
        }
    }
}

pub enum AudioType {
    WAV,
    MP3,
    OGG,
    FLAC,
}

pub struct AssetsFile {
    file_name: String,
    data: Option<Vec<u8>>,
    tmp_data: Arc<Mutex<Option<Vec<u8>>>>,
}

impl AssetsFile {
    pub fn new(file_name: &str) -> AssetsFile {
        AssetsFile {
            file_name: file_name.to_string(),
            data: None,
            tmp_data: Arc::new(Mutex::new(None)),
        }
    }

    pub fn load(&mut self) {
        #[cfg(any(target_arch = "asmjs", target_arch = "wasm32"))]
        {
            use stdweb::unstable::TryInto;
            use stdweb::web::event::ReadyStateChangeEvent;
            use stdweb::web::ArrayBuffer;
            use stdweb::web::IEventTarget;
            use stdweb::web::XhrReadyState;
            use stdweb::web::XhrResponseType;
            use stdweb::web::XmlHttpRequest;

            let data_clone = self.tmp_data.clone();

            let req = XmlHttpRequest::new();
            match req.open("GET", &self.file_name) {
                Ok(_) => (),
                Err(err) => eprintln!("{:?}", err),
            };
            if let Err(err) = req.set_response_type(XhrResponseType::ArrayBuffer) {
                eprintln!("{:?}", err);
            }

            req.add_event_listener(move |event: ReadyStateChangeEvent| {
                let req: XmlHttpRequest = js! {return @{event}.target}.try_into().unwrap();
                if req.ready_state() == XhrReadyState::Done {
                    if req.status() == 200 {
                        let array_buffer: ArrayBuffer = req.raw_response().try_into().unwrap();
                        let contents: Vec<u8> = Vec::from(array_buffer);
                        if let Ok(mut data) = data_clone.lock() {
                            *data = Some(contents);
                        }
                    }
                }
            });
            match req.send() {
                Ok(_) => (),
                Err(err) => println!("{:?}", err),
            };
        }

        #[cfg(not(any(target_arch = "asmjs", target_arch = "wasm32")))]
        {
            use std::fs::File;
            use std::io::Read;
            use std::thread;
            let data_clone = self.tmp_data.clone();
            let file_name = self.file_name.clone();
            thread::spawn(move || {
                let file_name = "./static/".to_owned() + &file_name;
                match File::open(file_name) {
                    Ok(mut file) => {
                        let mut contents = vec![];
                        match file.read_to_end(&mut contents) {
                            Ok(_) => {
                                match data_clone.lock() {
                                    Ok(mut data) => *data = Some(contents),
                                    Err(err) => eprintln!("{:?}", err),
                                };
                            }
                            Err(err) => eprintln!("{:?}", err),
                        };
                    }
                    Err(err) => eprintln!("{:?}", err),
                };
            });
        }
    }

    pub fn data(&mut self) -> Option<&Vec<u8>> {
        if self.data == None {
            if let Ok(mut tmp_data) = self.tmp_data.try_lock() {
                if tmp_data.is_some() {
                    //移出加载的临时文件数据
                    self.data = tmp_data.take();
                }
            }
        }
        self.data.as_ref()
    }
}

//生成指定范围的随即整数
pub fn rand_int(l: i32, b: i32) -> i32 {
    ((random() * (b as f64 - l as f64 + 1.0)).floor() + l as f64) as i32
}

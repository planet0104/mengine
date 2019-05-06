#![recursion_limit="128"]

#[cfg(any(target_arch = "asmjs", target_arch = "wasm32"))]
#[macro_use]
extern crate stdweb;

use std::time::{Duration, Instant};
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

pub use window::run;
pub use window::play_sound;

pub trait ImageLoader{
    fn load(&mut self, path:&str) -> Result<Rc<Image>, String>;
}

pub trait Graphics{
    fn clear_rect(&mut self, color:&[u8; 4], x:f64, y:f64, width:f64, height:f64);
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
    fn draw_image(&mut self, image:&Image, src:Option<&Rect<f64>>, dest:Option<&Rect<f64>>) -> Result<(), String>;
    /// 绘制文字
    ///
    /// # Arguments
    ///
    /// * `content`
    /// * `x`
    /// * `y`
    /// * `font_size` 字体 单位pt
    /// # Example
    ///
    /// ```
    /// g.draw_text("Hello!", 0., 20., &[255, 0, 0, 255], 16).expect("text draw failed.");
    /// ```
    fn draw_text(&mut self, cotnent:&str, x:f64, y:f64, color:&[u8; 4], font_size:u32) -> Result<(), String>;
}

#[derive(Debug)]
pub enum Event{
    MouseMove(i32, i32),
    KeyPress(String)
}

pub trait State: 'static{
    fn on_load(&mut self, image_loader:&mut ImageLoader);
    fn update(&mut self){}
    fn event(&mut self, _event:Event){}
    fn draw(&mut self, _graphics:&mut Graphics) -> Result<(), String>{
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

pub trait Image{
    fn as_any(&self) -> &dyn Any;
}

//计时器
pub struct Timer {
    frame_time: u64, //微妙
    start_time: Instant,
    next_time: Duration,
}

impl Timer {
    pub fn new(fps: u64) -> Timer {
        Timer {
            frame_time: 1_000_000 / fps,
            start_time: Instant::now(),
            next_time: Duration::from_millis(0),
        }
    }

    pub fn ready_for_next_frame(&mut self) -> bool {
        if self.start_time.elapsed() >= self.next_time {
            //更新时间
            self.next_time =
                self.start_time.elapsed() + Duration::from_micros(self.frame_time);
            true
        } else {
            false
        }
    }
}

pub struct SubImage{
    image: Rc<Image>,
    region: Rect<f64>,
}

impl SubImage {
    pub fn new(image: Rc<Image>, region: Rect<f64>) -> SubImage{
        SubImage {
            image,
            region
        }
    }

    pub fn draw(&self, g:&mut Graphics, dest:&Rect<f64>) -> Result<(), String>{
        g.draw_image(self.image.as_ref(), Some(&self.region), Some(dest))
    }
}

#[derive(Clone)]
pub struct Animation {
    image: Rc<Image>,
    frames: Vec<Rect<f64>>,
    current: usize,
    current_time: u32,
    frame_delay: u32,
    one_cycle: bool,
    active: bool,
}

impl Animation {
    pub fn new(image: Rc<Image>, frames:Vec<Rect<f64>>, frame_delay: u32) -> Animation{
        Animation {
            image,
            frames,
            current: 0,
            current_time: 0,
            frame_delay,
            one_cycle: false,
            active: false,
        }
    }

    pub fn active(image: Rc<Image>, frames:Vec<Rect<f64>>, frame_delay: u32) -> Animation{
        let mut anim = Self::new(image, frames, frame_delay);
        anim.start(0);
        anim
    }

    pub fn is_active(&self) -> bool{
        self.active
    }

    pub fn one_cycle(&mut self){
        self.one_cycle = true;
    }

    pub fn start(&mut self, current: usize){
        self.active = true;
        self.current = current;
        self.current_time = 0;
    }

    pub fn stop(&mut self, current: usize){
        self.active = false;
        self.current = current;
        self.current_time = 0;
    }

    pub fn is_end(&self) -> bool{
        self.current == self.frames.len()-1
    }

    /// Tick the animation forward by one step
    pub fn update(&mut self) {
        if self.active{
            self.current_time += 1;
            if self.current_time >= self.frame_delay {
                self.current += 1;
                self.current_time = 0;
                if self.current == self.frames.len(){
                    if self.one_cycle{
                        self.active = false;
                        self.current -= 1;
                    }else{
                        self.current = 0;
                    }
                }
            }
        }
    }

    pub fn draw(&self, g:&mut Graphics, dest:&Rect<f64>) -> Result<(), String>{
        g.draw_image(self.image.as_ref(), Some(&self.frames[self.current]), Some(dest))
    }

    // --/// Get the current frame of the animation
    // pub fn current_frame(&self) -> &Image {
    //     &self.frames[self.current]
    // }
}

use std::ops::{Sub, Add, AddAssign, SubAssign};
use std::cmp::PartialOrd;

#[derive(Clone, Debug)]
pub struct Rect<T: PartialOrd+Add+Sub+AddAssign+SubAssign+Copy+Default> {
    pub pos: Point<T>,
    pub size: Size<T>
}

impl <T: PartialOrd+Add<Output = T>+Sub<Output = T>+AddAssign+SubAssign+Copy+Default> Default for Rect<T> {
    fn default() -> Self {
        Rect{
            pos: Point::default(),
            size: Size::default()
        }
    }
}

impl <T: PartialOrd+Add<Output = T>+Sub<Output = T>+AddAssign+SubAssign+Copy+Default> Rect<T> {
    pub fn new(x: T, y: T, width: T, height: T) -> Rect<T> {
        Rect {
            pos: Point::new(x, y),
            size: Size::new(width, height)
        }
    }

    pub fn left(&self) -> T{
        self.pos.x
    }

    pub fn top(&self) -> T{
        self.pos.y
    }

    pub fn right(&self) -> T{
        self.pos.x+self.size.width
    }

    pub fn bottom(&self) -> T{
        self.pos.y+self.size.height
    }

    pub fn width(&self) -> T{
        self.size.width
    }

    pub fn height(&self) -> T{
        self.size.height
    }

    pub fn inflate(&mut self, dx: T, dy: T) {
        self.pos.x -= dx;
        self.size.width += dx+dx;
        self.pos.y -= dy;
        self.size.height += dy+dy;
    }

    pub fn offset(&mut self, dx: T, dy: T) {
        self.pos.x -= dx;
        self.pos.y -= dy;
    }

    pub fn move_to(&mut self, x: T, y:T){
        self.pos.x = x;
        self.pos.y = y;
    }

    pub fn contain(&self, x: T, y: T) -> bool {
        x >= self.pos.x && x <= self.right() && y >= self.pos.y && y <= self.bottom()
    }

    pub fn to_slice(&self) -> [T;4]{
        [self.pos.x, self.pos.y, self.size.width, self.size.height]
    }
}

#[derive(Clone, Debug, Copy)]
pub struct Point<T:Default> {
    pub x: T,
    pub y: T,
}

impl <T:Default> Point<T> {
    pub fn new(x: T, y:T) -> Point<T>{
        Point{x, y}
    }
}

impl <T:Default> Default for Point<T> {
    fn default() -> Self {
        Point{
            x: T::default(),
            y: T::default()
        }
     }
}

#[derive(Clone, Debug, Copy)]
pub struct Size<T:Default> {
    pub width: T,
    pub height: T,
}

impl <T:Default> Default for Size<T> {
    fn default() -> Self {
        Size{
            width: T::default(),
            height: T::default()
        }
     }
}

impl <T:Default> Size<T> {
    pub fn new(width: T, height:T) -> Size<T>{
        Size{width, height}
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
        }
    }
}

pub enum AudioType{
    WAV,
    MP3,
    OGG,
    FLAC
}

pub struct AssetsFile{
    file_name: String,
    data: Option<Vec<u8>>,
    tmp_data: Arc<Mutex<Option<Vec<u8>>>>
}

impl AssetsFile{
    pub fn new(file_name: &str) -> AssetsFile{
        AssetsFile{
            file_name: file_name.to_string(),
            data: None,
            tmp_data: Arc::new(Mutex::new(None)),
        }
    }

    pub fn load(&mut self){
        #[cfg(any(target_arch = "asmjs", target_arch = "wasm32"))]
        {
            use stdweb::web::XmlHttpRequest;
            use stdweb::web::XhrResponseType;
            use stdweb::web::event::ReadyStateChangeEvent;
            use stdweb::web::IEventTarget;
            use stdweb::web::XhrReadyState;
            use stdweb::traits::IEvent;
            use stdweb::unstable::TryInto;
            use stdweb::web::ArrayBuffer;
            
            let data_clone = self.tmp_data.clone();

            let mut req = XmlHttpRequest::new();
            match req.open("GET", &self.file_name){
                Ok(_) => (),
                Err(err) => println!("{:?}", err)
            };
            req.set_response_type(XhrResponseType::ArrayBuffer);
            req.add_event_listener(move |event: ReadyStateChangeEvent|{
                let req:XmlHttpRequest = js!{return @{event}.target}.try_into().unwrap();
                if req.ready_state() == XhrReadyState::Done{
                    if req.status() == 200{
                        let array_buffer:ArrayBuffer = req.raw_response().try_into().unwrap();
                        let contents:Vec<u8> = Vec::from(array_buffer);
                        if let Ok(mut data) = data_clone.lock(){
                            *data = Some(contents);
                        }
                    }
                }
            });
            match req.send(){
                Ok(_) => (),
                Err(err) => println!("{:?}", err)
            };
        }

        #[cfg(not(any(target_arch = "asmjs", target_arch = "wasm32")))]{
            use std::thread;
            use std::fs::File;
            use std::io::Read;
            let data_clone = self.tmp_data.clone();
            let file_name = self.file_name.clone();
            thread::spawn(move || {
                let file_name = "./static/".to_owned()+&file_name;
                match File::open(file_name){
                    Ok(mut file) => {
                        let mut contents = vec![];
                        match file.read_to_end(&mut contents){
                            Ok(_) => {
                                match data_clone.lock(){
                                    Ok(mut data) => *data = Some(contents),
                                    Err(err) => eprintln!("{:?}", err)
                                };
                            },
                            Err(err) => eprintln!("{:?}", err)
                        };
                    },
                    Err(err) => eprintln!("{:?}", err)
                };
            });
        }
    }

    pub fn data(&mut self) -> Option<&Vec<u8>>{
        if self.data == None{
            if let Ok(mut tmp_data) = self.tmp_data.try_lock(){
                if tmp_data.is_some(){
                    //移出加载的临时文件数据
                    self.data = tmp_data.take();
                }
            }
        }
        self.data.as_ref()
    }
}

#[cfg(not(any(target_arch = "asmjs", target_arch = "wasm32")))]
pub fn log<T: std::fmt::Debug>(s:T){
    println!("{:?}", s);
}

#[cfg(any(target_arch = "asmjs", target_arch = "wasm32"))]
pub fn log<T: std::fmt::Debug>(s:T){
    console!(log, format!("{:?}", s));
}
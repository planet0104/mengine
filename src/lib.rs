#[cfg(target_arch = "asmjs")]
#[macro_use]
extern crate stdweb;

use std::time::{Duration, Instant};
use std::any::Any;
use std::rc::Rc;

#[cfg(not(target_arch = "asmjs"))]
mod pc;
#[cfg(not(target_arch = "asmjs"))]
use pc as window;

#[cfg(target_arch = "asmjs")]
mod web;
#[cfg(target_arch = "asmjs")]
use web as window;

pub use window::run;

pub trait ImageLoader{
    fn load(&mut self, path:&str) -> Result<Rc<Image>, String>;
}

pub trait Graphics{
    fn clear_rect(&mut self, color:&[u8; 4], x:f64, y:f64, width:f64, height:f64);
    fn draw_image(&mut self, image:&Image, src:Option<[f64; 4]>, dest:Option<[f64; 4]>) -> Result<(), String>;
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
    #[cfg(target_arch = "asmjs")]
    fn handle_error(&mut self, error: String) {
        console!(error, error);
    }
    #[cfg(not(target_arch = "asmjs"))]
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

pub struct Animation {
    image: Rc<Image>,
    frames: Vec<[f64; 4]>,
    current: usize,
    current_time: u32,
    frame_delay: u32
}

impl Animation {
    pub fn new(image: Rc<Image>, frames:Vec<[f64;4]>, frame_delay: u32) -> Animation{
        Animation {
            image,
            frames,
            current: 0,
            current_time: 0,
            frame_delay
        }
    }

    /// Tick the animation forward by one step
    pub fn update(&mut self) {
        self.current_time += 1;
        if self.current_time >= self.frame_delay {
            self.current = (self.current + 1) % self.frames.len();
            self.current_time = 0;
        }
    }

    pub fn draw(&self, g:&mut Graphics, dest:[f64; 4]) -> Result<(), String>{
        g.draw_image(self.image.as_ref(), Some(self.frames[self.current]), Some(dest))
    }

    // --/// Get the current frame of the animation
    // pub fn current_frame(&self) -> &Image {
    //     &self.frames[self.current]
    // }
}
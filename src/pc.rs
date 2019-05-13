use piston_window::{UpdateEvent, EventSettings, EventLoop, ImageSize, PressEvent, Button, Transformed, Filter, Glyphs, Text, rectangle, Image as GfxImage, TextureSettings, Flip, Texture, Context, G2d, PistonWindow, WindowSettings};
use super::{AudioType, Settings, Event, ImageLoader, Image, Graphics, State};
use std::path::Path;
use std::any::Any;
use std::cell::RefCell;
use piston_window::mouse::{MouseButton, MouseCursorEvent};
use piston_window::keyboard::Key;
use std::rc::Rc;
use winit::Icon;

struct TextureLoader<'a>{
    window: &'a mut PistonWindow
}
impl <'a> ImageLoader for TextureLoader<'a>{
    fn load(&mut self, path:&str) -> Result<Rc<Image>, String>{
        let path = "./static/".to_owned()+path;
        let texture = Texture::from_path(&mut self.window.factory, Path::new(&path), Flip::None, &TextureSettings::new())?;
        Ok(Rc::new(PcImage{width: texture.get_width(), height: texture.get_height(), texture}))
    }
}

struct PcImage{
    width: u32,
    height: u32,
    texture: gfx_texture::Texture<gfx_device_gl::Resources>
}
impl Image for PcImage{
    fn width(&self) -> u32{
        self.width
    }
    fn height(&self) -> u32{
        self.height
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct PistonGraphics<'a, 'b>{
    glyphs: Option<&'a RefCell<Glyphs>>,
    context: Context,
    graphics: &'a mut G2d<'b>,
}

impl <'a, 'b> Graphics for PistonGraphics<'a, 'b>{
    fn clear_rect(&mut self, color:&[u8; 4], x:f64, y:f64, width:f64, height:f64){
        rectangle([color[0] as f32/255.0, color[1] as f32/255.0, color[2] as f32/255.0, color[3] as f32/255.0], // red
                [x, y, width, height],
                self.context.transform,
                self.graphics);
    }

    fn draw_image(&mut self, image:&Image, src:Option<[f64; 4]>, dest:Option<[f64; 4]>) -> Result<(), String>{
        match image.as_any().downcast_ref::<PcImage>(){
            Some(image) => {
                let mut gfx_image  = GfxImage::new();
                if let Some(src) = src{
                    gfx_image = gfx_image.src_rect(src);
                }
                if let Some(dest) = dest{
                    gfx_image = gfx_image.rect(dest);
                }
                gfx_image.draw(&image.texture, &self.context.draw_state, self.context.transform, self.graphics);
                Ok(())
            },
            None => Err("Image downcast PcImage Error!".to_string())
        }
    }

    fn draw_text(&mut self, cotnent:&str, x:f64, y:f64, color:&[u8; 4], font_size:u32) -> Result<(), String>{
        if let Some(glyphs) = self.glyphs.as_mut(){
            let text = Text::new_color([color[0] as f32/255.0, color[1] as f32/255.0, color[2] as f32/255.0, color[3] as f32/255.0], font_size);
            match text.draw(cotnent,
                    &mut *glyphs.borrow_mut(),
                    &self.context.draw_state,
                    self.context.trans(x, y).transform,
                    self.graphics){
                        Err(err) => Err(format!("{:?}", err)),
                        Ok(()) => Ok(())
                    }
        }else{
            Ok(())
        }
    }
}

/// 启动线程播放声音
pub fn play_sound(data:&[u8], _t:AudioType){
    use std::thread;
    let data = data.to_vec();
    thread::spawn(move || {
        use std::io::Cursor;
        let device = rodio::default_output_device();
        if device.is_none(){
            eprintln!("no default output device.");
            return;
        }
        let sink = rodio::Sink::new(&device.unwrap());
        let decoder = rodio::Decoder::new(Cursor::new(data.to_vec()));
        if decoder.is_err(){
            eprintln!("{:?}", decoder.err());
            return;
        }
        sink.append(decoder.unwrap());
        sink.sleep_until_end();
    });
}

pub fn run<S:State>(title:&str, width:f64, height:f64, settings: Settings){
    //第一次启动窗口不移动鼠标也会触发一次mouse move事件，过滤这个事件
    let mut got_first_mouse_event = false;
    let mut window:PistonWindow = WindowSettings::new(title, [width, height]).exit_on_esc(true).build().unwrap();
    window.set_event_settings(EventSettings{ups: settings.ups, ..Default::default()});
    let mut texture_loader = TextureLoader{window:&mut window};

    let mut state = S::new(&mut texture_loader);

    let mut glyphs = None;
    if let Some(font) = settings.font_file{
        let font = "./static/".to_owned()+font;
        match Glyphs::new(font.clone(), window.factory.clone(), TextureSettings::new().filter(Filter::Nearest)){
            Ok(g) => glyphs = Some(RefCell::new(g)),
            Err(err) => state.handle_error(format!("font load failed! {} {:?}", font, err))
        };
    }

    if let Some(path) = settings.icon_path{
        let path = "./static/".to_owned()+path;
        let icon = Icon::from_path(path).unwrap();
        window.window.window.set_window_icon(Some(icon));
    }

    let mut mouse_pos = [0.0; 2];

    while let Some(event) = window.next() {
        event.update(|_u|{
            state.update();
        });
        window.draw_2d(&event, |context, graphics| {
            match state.draw(&mut PistonGraphics{glyphs: glyphs.as_ref(), context: context, graphics: graphics}){
                Ok(()) => (),
                Err(err) => state.handle_error(format!("font load failed! {:?}", err))
            };
        });
        event.mouse_cursor(|x, y| {
            if got_first_mouse_event{
                mouse_pos[0] = x;
                mouse_pos[1] = y;
                state.event(Event::MouseMove(x, y));
            }else{
                got_first_mouse_event = true;
            }
        });
        if let Some(Button::Mouse(mouse)) = event.press_args() {
            match mouse{
                MouseButton::Left => state.event(Event::Click(mouse_pos[0], mouse_pos[1])),
                _ => ()
            };
        }
        if let Some(Button::Keyboard(key)) = event.press_args() {
            match key{
                Key::D0 |
                Key::D1 |
                Key::D2 |
                Key::D3 |
                Key::D4 |
                Key::D5 |
                Key::D6 |
                Key::D7 |
                Key::D8 |
                Key::D9 |
                Key::NumPad0 |
                Key::NumPad1 |
                Key::NumPad2 |
                Key::NumPad3 |
                Key::NumPad4 |
                Key::NumPad5 |
                Key::NumPad6 |
                Key::NumPad7 |
                Key::NumPad8 |
                Key::NumPad9 => {
                    let key = format!("{:?}", key);
                    state.event(Event::KeyPress(key.replace("D", "").replace("NumPad", "")))
                }
                _ => state.event(Event::KeyPress(format!("{:?}", key)))
            };
        };
    }
}
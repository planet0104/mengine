use stdweb::traits::*;
use stdweb::unstable::TryInto;
use stdweb::web::html_element::ImageElement;
use stdweb::web::IElement;
use std::ffi::OsStr;
use std::path::Path;
use std::rc::Rc;
use stdweb::web::{
    document,
    window,
    CanvasRenderingContext2d
};

use stdweb::web::event::{
    MouseMoveEvent,
    MouseDownEvent,
    KeyPressEvent,
    KeyDownEvent,
    KeyUpEvent,
};
use stdweb::web::html_element::CanvasElement;
use super::{AudioType, Settings, Rect, Event, Image, ImageLoader, Graphics, State};
use std::cell::RefCell;
use std::any::Any;

struct WebImageLoader{}
impl ImageLoader for WebImageLoader{
    fn load(&mut self, path:&str) -> Result<Rc<Image>, String>{
        let image = ImageElement::new();
        image.set_src(path);
        Ok(Rc::new(WebImage{image}))
    }
}

struct WebImage{
    image: ImageElement
}
impl WebImage{
    
}
impl Image for WebImage{
    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct HTMLGraphics{
    font_family: String,
    context: CanvasRenderingContext2d,
}

impl Graphics for HTMLGraphics{
    fn clear_rect(&mut self, color:&[u8; 4], x:f64, y:f64, width:f64, height:f64){
        self.context.set_fill_style_color(&format!("rgba({},{},{},{})", color[0], color[1], color[2], color[3]));
        self.context.fill_rect(x, y, width, height);
    }

    fn draw_image(&mut self, image:&Image, src:Option<&Rect<f64>>, dest:Option<&Rect<f64>>) -> Result<(), String>{
        match image.as_any().downcast_ref::<WebImage>(){
            Some(image) => {
                match if src.is_none() && dest.is_none(){
                    self.context.draw_image(image.image.clone(), 0., 0.)
                }else if src.is_none() && dest.is_some(){
                    let dest = dest.unwrap();
                    self.context.draw_image_d(image.image.clone(), dest.pos.x, dest.pos.y, dest.size.width, dest.size.height)
                }else if src.is_some() && dest.is_none(){
                    let src = src.unwrap();
                    self.context.draw_image_s(image.image.clone(), src.pos.x, src.pos.y, src.size.width, src.size.height, 0., 0., image.image.width().into(), image.image.height().into())
                }else{
                    let src = src.unwrap();
                    let dest = dest.unwrap();
                    self.context.draw_image_s(image.image.clone(), src.pos.x, src.pos.y, src.size.width, src.size.height, dest.pos.x, dest.pos.y, dest.size.width, dest.size.height)
                }{
                    Err(err) => Err(format!("{:?}", err)),
                    Ok(_) => Ok(())
                }
            },
            None => Err("Image downcast PcImage Error!".to_string())
        }
    }

    fn draw_text(&mut self, cotnent:&str, x:f64, y:f64, color:&[u8; 4], font_size:u32) -> Result<(), String>{
        self.context.set_fill_style_color(&format!("rgba({},{},{},{})", color[0], color[1], color[2], color[3]));
        self.context.set_font(&format!("{}pt {}", font_size, self.font_family));
        self.context.fill_text(cotnent, x, y, None);
        Ok(())
    }
}

thread_local! {
    static UPDATE_RATE:RefCell<u64> = RefCell::new(30);
    static GRAPHICS: RefCell<HTMLGraphics> = {
        let canvas:CanvasElement = document().query_selector("#canvas").unwrap().unwrap().try_into().unwrap();
        RefCell::new(HTMLGraphics{font_family:"Arial".to_string(), context: canvas.get_context().unwrap() })
    };
    static STATE:RefCell<Option<Box<State>>> = RefCell::new(None);
}

fn update_game(){
    STATE.with(|state|{ state.borrow_mut().as_mut().unwrap().update(); });
    window().set_timeout(update_game, UPDATE_RATE.with(|rate|{ (1000.0 / *rate.borrow() as f64) as u32 }));
}

//request_animation_frame
fn request_animation_frame(_timestamp:f64){
    GRAPHICS.with(|graphics|{
        STATE.with(|state|{
            let mut state = state.borrow_mut();
            let state = state.as_mut().expect("state borrow error!");
            match state.draw(&mut *graphics.borrow_mut()){
                Err(err) => state.handle_error(format!("draw {:?}", err)),
                Ok(()) => ()
            };
        });
    });
    window().request_animation_frame(request_animation_frame);
}

pub fn play_sound(data:&[u8], _t:AudioType){
    js!{
        let bytes = new Uint8Array(@{data}).buffer;
        var audioCtx = window.audioContext;
        audioCtx.decodeAudioData(bytes, function(buffer){
            try{
                var source = audioCtx.createBufferSource();
                source.buffer = buffer;
                source.connect(audioCtx.destination);
                source.start(0);
            }catch(e){
                console.log(e);
            }
        });
    };
}

pub fn run<S:State>(title: &str, width:f64, height:f64, settings: Settings, mut state:S){
    document().body().expect("no html body!!").append_html("<canvas id=\"canvas\"></canvas>").expect("append canvas fail!!");
    state.on_load(&mut WebImageLoader{});

    //声音播放
    js!{
        const AudioContext = window.AudioContext || window.webkitAudioContext;
        window.audioContext = new AudioContext();
        window.audioContextResume = false;
    };
    UPDATE_RATE.with(|rate|{ *rate.borrow_mut() = settings.ups; });

    if let Some(icon) = settings.icon_path{
        let mut icon_link = String::from("<link rel=\"icon\" type=\"image/_type_\" href=\"_path_\" />");
        icon_link = icon_link.replace("_path_", icon);
        let icon_path = Path::new(icon);
        if let Some(ext) = icon_path.extension(){    
            if ext == "ico"{
                icon_link = icon_link.replace("_type_", "x-icon");
            }else{
                icon_link = icon_link.replace("_type_", ext.to_str().unwrap_or("*"));
            }
        }else{
            icon_link = icon_link.replace("_type_", "*");
        }
        if let Some(head) = document().head(){
            let _ = head.append_html(&icon_link);
            icon_link = icon_link.replace("rel=\"icon\"", "rel=\"shortcut icon\"");
            let _ = head.append_html(&icon_link);
        }
    }

    if let Some(font) = settings.font_file{
        match ||->Result<(), Box<std::error::Error>>{
            let path = Path::new(font);
            let stem = path.file_stem().unwrap_or(OsStr::new("MyFont")).to_str().unwrap_or("MyFont");
            let mut style = String::new();
            style.push_str("<style>@font-face{ font-family: ");
            GRAPHICS.with(|graphics|{ graphics.borrow_mut().font_family = stem.to_string() });
            style.push_str(stem);
            style.push_str("; src: url('");
            style.push_str(font);
            style.push_str("');}</style>");
            if let Some(head) = document().head(){
                head.append_html(&style)?;
            }else{
                document().body().expect("no html body!").append_html(&style)?;
            }
            Ok(())
        }(){
            Ok(_) => (),
            Err(err) => state.handle_error(format!("font load error {:?}", err))
        }
    }

    STATE.with(|s|{ *s.borrow_mut() = Some(Box::new(state)); });

    //init
    match ||->Result<(), Box<std::error::Error>>{
        let element = document().query_selector("#canvas")?;
        if element.is_none(){
            STATE.with(|state|{ state.borrow_mut().as_mut().unwrap().handle_error("canvas is None!".to_string()); });
            return Ok(());
        }
        let canvas: CanvasElement = element.unwrap().try_into()?;
        canvas.set_width(width as u32);
        canvas.set_height(height as u32);
        canvas.set_attribute("style", &format!("width:{}px;height:{}px", width, height))?;
        document().set_title(title);
        window().request_animation_frame(request_animation_frame);
        window().set_timeout(update_game, 0);

        canvas.add_event_listener(move |event: MouseMoveEvent| {
            STATE.with(|state|{ state.borrow_mut().as_mut().unwrap().event(Event::MouseMove(event.client_x(), event.client_y())); });
        });
        canvas.add_event_listener(move |event: MouseDownEvent| {
            js!{
                if (window.audioContext.state !== "running" && !window.audioContextResume) {
                    window.audioContext.resume();
                    window.audioContextResume = true;
                    console.log("AudioContextResume.");
                }
            };
        });
        // document().add_event_listener(move |event: KeyPressEvent| {
        //     event.prevent_default();
        //     STATE.with(|state|{ state.borrow_mut().as_mut().unwrap().event(Event::KeyPress(event.key().to_uppercase())); });
        // });
        document().add_event_listener(move |event: KeyDownEvent| {
            event.prevent_default();
            STATE.with(|state|{ state.borrow_mut().as_mut().unwrap().event(Event::KeyPress(event.key().to_uppercase())); });
        });
        Ok(())
    }(){
        Ok(_) => (),
        Err(err) => STATE.with(|state|{ state.borrow_mut().as_mut().unwrap().handle_error(format!("init error {:?}", err)); })
    }
}
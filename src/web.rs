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
    ClickEvent,
    KeyDownEvent,
    KeyUpEvent,
};
use stdweb::web::html_element::CanvasElement;
use super::{AudioType, Settings, Event, Image, ImageLoader, Graphics, State};
use std::cell::RefCell;
use std::any::Any;

struct WebImageLoader{}
impl ImageLoader for WebImageLoader{
    fn load(&mut self, path:&str) -> Result<Rc<Image>, String>{
        let image = ImageElement::new();
        let web_image = Rc::new(WebImage{image});
        web_image.image.set_src(path);
        Ok(web_image)
    }
}

struct WebImage{
    image: ImageElement
}
impl Image for WebImage{
    fn width(&self) -> u32{
        self.image.width()
    }
    fn height(&self) -> u32{
        self.image.height()
    }
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

    fn draw_image(&mut self, image:&Image, src:Option<[f64; 4]>, dest:Option<[f64; 4]>) -> Result<(), String>{
        match image.as_any().downcast_ref::<WebImage>(){
            Some(image) => {
                match if src.is_none() && dest.is_none(){
                    self.context.draw_image(image.image.clone(), 0., 0.)
                }else if src.is_none() && dest.is_some(){
                    let dest = dest.unwrap();
                    self.context.draw_image_d(image.image.clone(), dest[0], dest[1], dest[2], dest[3])
                }else if src.is_some() && dest.is_none(){
                    let src = src.unwrap();
                    self.context.draw_image_s(image.image.clone(), src[0], src[1], src[2], src[3], 0., 0., image.image.width().into(), image.image.height().into())
                }else{
                    let src = src.unwrap();
                    let dest = dest.unwrap();
                    self.context.draw_image_s(image.image.clone(), src[0], src[1], src[2], src[3], dest[0], dest[1], dest[2], dest[3])
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

pub fn run<S:State>(title: &str, width:f64, height:f64, settings: Settings){
    document().body().expect("no html body!!").append_html("<canvas id=\"canvas\"></canvas>").expect("append canvas fail!!");

    let canvas:CanvasElement = document().query_selector("#canvas").unwrap().unwrap().try_into().unwrap();
    let mut graphics = HTMLGraphics{font_family:"Arial".to_string(), context: canvas.get_context().unwrap() };

    let state = Rc::new(RefCell::new(S::new(&mut WebImageLoader{})));

    //声音播放
    js!{
        const AudioContext = window.AudioContext || window.webkitAudioContext;
        window.audioContext = new AudioContext();
        window.audioContextResume = false;
    };

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
            graphics.font_family = stem.to_string();
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
            Err(err) => state.borrow_mut().handle_error(format!("font load error {:?}", err))
        }
    }

    //init
    match ||->Result<(), Box<std::error::Error>>{
        let element = document().query_selector("#canvas")?;
        if element.is_none(){
            state.borrow_mut().handle_error("canvas is None!".to_string());
            return Ok(());
        }
        let canvas: CanvasElement = element.unwrap().try_into()?;
        canvas.set_width(width as u32);
        canvas.set_height(height as u32);
        canvas.set_attribute("style", &format!("width:{}px;height:{}px", width, height))?;
        document().set_title(title);

        // request_animation_frame
        let s_animation = state.clone();
        let animation_fn = move ||{
            let mut state = s_animation.borrow_mut();
            if let Err(err) = state.draw(&mut graphics){
                state.handle_error(format!("draw error {:?}", err));
            }
        };

        js!{
            var animation_fn = @{animation_fn};
            window.request_animation_frame_fn = function(timestamp){
                animation_fn();
                requestAnimationFrame(window.request_animation_frame_fn);
            };
            requestAnimationFrame(window.request_animation_frame_fn);
        };

        // update

        let s_update = state.clone();
        let update_fn = move ||{
            s_update.borrow_mut().update();
        };
        update_fn();

        let delay = (1000.0 / settings.ups as f64) as u32;
        js!{
            var delay = @{delay};
            var update_callback = @{update_fn};
            function game_update(){
                update_callback();
                setTimeout(game_update, delay);
            }
            setTimeout(game_update, 0);
        };

        let s_mouse_move = state.clone();
        canvas.add_event_listener(move |event: MouseMoveEvent| {
            s_mouse_move.borrow_mut().event(Event::MouseMove(event.offset_x(), event.offset_y()));
        });

        let s_click = state.clone();
        canvas.add_event_listener(move |event: ClickEvent| {
            s_click.borrow_mut().event(Event::Click(event.offset_x(), event.offset_y()));
        });
        
        canvas.add_event_listener(move |_event: MouseDownEvent| {
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
        let s_key_down = state.clone();
        document().add_event_listener(move |event: KeyDownEvent| {
            event.prevent_default();
            s_key_down.borrow_mut().event(Event::KeyPress(event.key().to_uppercase()));
        });
        Ok(())
    }(){
        Ok(_) => (),
        Err(err) => state.borrow_mut().handle_error(format!("init error {:?}", err))
    }
}
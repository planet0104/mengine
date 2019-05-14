use std::ffi::OsStr;
use std::path::Path;
use std::rc::Rc;
use stdweb::traits::*;
use stdweb::unstable::TryInto;
use stdweb::web::html_element::ImageElement;
use stdweb::web::IElement;
use stdweb::web::{document, window, CanvasRenderingContext2d};

use super::{AudioType, Event, Graphics, Image, ImageLoader, Settings, State};
use std::any::Any;
use std::cell::RefCell;
use stdweb::web::event::{
    // KeyPressEvent,
    ClickEvent,
    // KeyUpEvent,
    ITouchEvent,
    KeyDownEvent,
    MouseDownEvent,
    PointerMoveEvent,
    ResizeEvent,
    TouchMove,
};
use stdweb::web::html_element::CanvasElement;

struct WebImageLoader {}
impl ImageLoader for WebImageLoader {
    fn load(&mut self, path: &str) -> Result<Rc<Image>, String> {
        let image = ImageElement::new();
        let web_image = Rc::new(WebImage { image });
        web_image.image.set_src(path);
        Ok(web_image)
    }
}

struct WebImage {
    image: ImageElement,
}
impl Image for WebImage {
    fn width(&self) -> u32 {
        self.image.width()
    }
    fn height(&self) -> u32 {
        self.image.height()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct HTMLGraphics {
    font_family: String,
    context: CanvasRenderingContext2d,
}

impl Graphics for HTMLGraphics {
    fn clear_rect(&mut self, color: &[u8; 4], x: f64, y: f64, width: f64, height: f64) {
        self.context.set_fill_style_color(&format!(
            "rgba({},{},{},{})",
            color[0], color[1], color[2], color[3]
        ));
        self.context.fill_rect(x, y, width, height);
    }

    fn draw_image(
        &mut self,
        image: &Image,
        src: Option<[f64; 4]>,
        dest: Option<[f64; 4]>,
    ) -> Result<(), String> {
        match image.as_any().downcast_ref::<WebImage>() {
            Some(image) => {
                match if src.is_none() && dest.is_none() {
                    self.context.draw_image(image.image.clone(), 0., 0.)
                } else if src.is_none() && dest.is_some() {
                    let dest = dest.unwrap();
                    self.context.draw_image_d(
                        image.image.clone(),
                        dest[0],
                        dest[1],
                        dest[2],
                        dest[3],
                    )
                } else if src.is_some() && dest.is_none() {
                    let src = src.unwrap();
                    self.context.draw_image_s(
                        image.image.clone(),
                        src[0],
                        src[1],
                        src[2],
                        src[3],
                        0.,
                        0.,
                        image.image.width().into(),
                        image.image.height().into(),
                    )
                } else {
                    let src = src.unwrap();
                    let dest = dest.unwrap();
                    self.context.draw_image_s(
                        image.image.clone(),
                        src[0],
                        src[1],
                        src[2],
                        src[3],
                        dest[0],
                        dest[1],
                        dest[2],
                        dest[3],
                    )
                } {
                    Err(err) => Err(format!("{:?}", err)),
                    Ok(_) => Ok(()),
                }
            }
            None => Err("Image downcast PcImage Error!".to_string()),
        }
    }

    fn draw_text(
        &mut self,
        cotnent: &str,
        x: f64,
        y: f64,
        color: &[u8; 4],
        font_size: u32,
    ) -> Result<(), String> {
        self.context.set_fill_style_color(&format!(
            "rgba({},{},{},{})",
            color[0],
            color[1],
            color[2],
            color[3] as f64 / 255.0
        ));
        self.context
            .set_font(&format!("{}px {}", font_size, self.font_family));
        self.context.fill_text(cotnent, x, y, None);
        Ok(())
    }
}

pub fn play_sound(data: &[u8], _t: AudioType) {
    js! {
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

pub fn run<S: State>(title: &str, width: f64, height: f64, settings: Settings) {
    document()
        .body()
        .expect("no html body!!")
        .append_html("<canvas id=\"canvas\"></canvas>")
        .expect("append canvas fail!!");

    let canvas: CanvasElement = document()
        .query_selector("#canvas")
        .unwrap()
        .unwrap()
        .try_into()
        .unwrap();
    let mut graphics = HTMLGraphics {
        font_family: "Arial".to_string(),
        context: canvas.get_context().unwrap(),
    };

    let state = Rc::new(RefCell::new(S::new(&mut WebImageLoader {})));
    let (trans_x, trans_y) = (Rc::new(RefCell::new(0.0)), Rc::new(RefCell::new(0.0)));
    let (scale_x, scale_y) = (Rc::new(RefCell::new(1.0)), Rc::new(RefCell::new(1.0)));

    //声音播放
    js! {
        const AudioContext = window.AudioContext || window.webkitAudioContext;
        window.audioContext = new AudioContext();
        window.audioContextResume = false;
    };

    use askama::Template;

    #[derive(Template)]
    #[template(path = "head_part.html")]
    struct Context<'a> {
        icon_type: &'a str,
        icon_href: &'a str,
        background_color: &'a str,
        font_family: &'a str,
        font_src: &'a str,
    }

    //添加head
    if let Some(head) = document().head() {
        //获取icon文件路径和扩展名
        let (icon_path, icon_type) = if let Some(icon) = settings.icon_path {
            let icon_path = Path::new(icon);
            let mut icon_type = "*";
            if let Some(ext) = icon_path.extension() {
                if ext == "ico" {
                    icon_type = "x-icon";
                } else {
                    icon_type = ext.to_str().unwrap_or("*");
                }
            }
            (icon_path.to_str().unwrap_or(""), icon_type)
        } else {
            ("", "*")
        };

        let (font_family, font_src) = if let Some(font) = settings.font_file {
            || -> Result<(&str, &str), Box<std::error::Error>> {
                let path = Path::new(font);
                let stem = path
                    .file_stem()
                    .unwrap_or(OsStr::new("MyFont"))
                    .to_str()
                    .unwrap_or("MyFont");
                let mut style = String::new();
                style.push_str("<style>@font-face{ font-family: ");
                graphics.font_family = stem.to_string();
                Ok((stem, font))
            }()
            .unwrap_or(("", ""))
        } else {
            ("", "")
        };

        let mut background_color = "".to_string();
        if let Some(color) = settings.background_color {
            background_color = format!(
                "rgba({},{},{},{})",
                color[0],
                color[1],
                color[2],
                color[3] as f64 / 255.0
            );
        };

        let context = Context {
            background_color: &background_color,
            icon_type: icon_type,
            icon_href: icon_path,
            font_family,
            font_src,
        };

        match context.render() {
            Ok(rendered) => {
                let _ = head.append_html(&rendered);
            }
            Err(err) => log(format!("render error:{:?}", err)),
        };
    }

    //init
    match || -> Result<(), Box<std::error::Error>> {
        let window = window();
        let element = document().query_selector("#canvas")?;
        if element.is_none() {
            state
                .borrow_mut()
                .handle_error("canvas is None!".to_string());
            return Ok(());
        }
        let canvas: CanvasElement = element.unwrap().try_into()?;
        log(format!(
            "window.inner_width()={}, window.inner_height()={}",
            window.inner_width(),
            window.inner_height()
        ));
        canvas.set_width(window.inner_width() as u32);
        canvas.set_height(window.inner_height() as u32);
        canvas.set_attribute(
            "style",
            &format!(
                "width:{}px;height:{}px",
                window.inner_width(),
                window.inner_height()
            ),
        )?;
        document().set_title(title);

        //随窗口更改canvas大小
        window.add_event_listener(|_event: ResizeEvent| {
            let canvas: CanvasElement = document()
                .query_selector("#canvas")
                .unwrap()
                .unwrap()
                .try_into()
                .unwrap();
            let window = stdweb::web::window();
            canvas.set_width(window.inner_width() as u32);
            canvas.set_height(window.inner_height() as u32);
            let _ = canvas.set_attribute(
                "style",
                &format!(
                    "width:{}px;height:{}px",
                    window.inner_width(),
                    window.inner_height()
                ),
            );
        });

        // request_animation_frame
        let s_animation = state.clone();
        let (auto_scale, draw_center) = (settings.auto_scale, settings.draw_center);
        let background_color = settings.background_color;
        let (tx_clone, ty_clone, sx_clone, sy_clone) = (
            trans_x.clone(),
            trans_y.clone(),
            scale_x.clone(),
            scale_y.clone(),
        );
        let mut animation_fn = move |_timestamp| {
            let (window_width, window_height) =
                (window.inner_width() as f64, window.inner_height() as f64);
            let mut state = s_animation.borrow_mut();
            if let Some(color) = background_color {
                graphics.clear_rect(&color, 0., 0., window_width, window_height);
            }
            graphics.context.save();

            let (mut new_width, mut new_height) = (width, height);
            let (mut scale_x, mut scale_y) = (1.0, 1.0);
            if auto_scale {
                //画面不超过窗口高度
                new_height = window_height;
                new_width = new_height / height * width;

                if new_width > window_width {
                    new_width = window_width;
                    new_height = new_width / width * height;
                }
                scale_x = new_width / width;
                scale_y = new_height / height;
                graphics.context.scale(scale_x, scale_y);
                *sx_clone.borrow_mut() = scale_x;
                *sy_clone.borrow_mut() = scale_y;
            }
            if draw_center {
                let trans_x = (window_width - new_width) / 2.;
                let trans_y = (window_height - new_height) / 2.;
                graphics
                    .context
                    .translate(trans_x / scale_x, trans_y / scale_y);
                *tx_clone.borrow_mut() = trans_x;
                *ty_clone.borrow_mut() = trans_y;
            }

            if let Err(err) = state.draw(&mut graphics) {
                state.handle_error(format!("draw error {:?}", err));
            }
            if draw_center {
                graphics.context.restore();
            }
        };
        animation_fn(0.0);

        // update
        let s_update = state.clone();
        let mut timer = super::AnimationTimer::new(settings.ups as f64);
        js! {setInterval(@{move ||{
                if timer.ready_for_next_frame(){
                    s_update.borrow_mut().update();
                }
            }}, 1);//频率最高220~230
        };

        js! {
            var animation_fn = @{animation_fn};
            window.request_animation_frame_fn = function(timestamp){
                animation_fn(timestamp);
                requestAnimationFrame(window.request_animation_frame_fn);
            };
            requestAnimationFrame(window.request_animation_frame_fn);
        };

        let s_mouse_move = state.clone();
        let (tx_clone, ty_clone, sx_clone, sy_clone) = (
            trans_x.clone(),
            trans_y.clone(),
            scale_x.clone(),
            scale_y.clone(),
        );
        canvas.add_event_listener(move |event: PointerMoveEvent| {
            s_mouse_move.borrow_mut().event(Event::MouseMove(
                (event.offset_x() - *tx_clone.borrow()) / *sx_clone.borrow(),
                (event.offset_y() - *ty_clone.borrow()) / *sy_clone.borrow(),
            ));
        });

        let s_touch_move = state.clone();
        let (tx_clone, ty_clone, sx_clone, sy_clone) = (
            trans_x.clone(),
            trans_y.clone(),
            scale_x.clone(),
            scale_y.clone(),
        );
        canvas.add_event_listener(move |event: TouchMove| {
            let touchs = event.target_touches();
            if touchs.len() > 0 {
                s_touch_move.borrow_mut().event(Event::MouseMove(
                    (touchs[0].client_x() - *tx_clone.borrow()) / *sx_clone.borrow(),
                    (touchs[0].client_y() - *ty_clone.borrow()) / *sy_clone.borrow(),
                ));
            }
        });

        let s_click = state.clone();
        let (tx_clone, ty_clone, sx_clone, sy_clone) = (
            trans_x.clone(),
            trans_y.clone(),
            scale_x.clone(),
            scale_y.clone(),
        );
        canvas.add_event_listener(move |event: ClickEvent| {
            s_click.borrow_mut().event(Event::Click(
                (event.offset_x() - *tx_clone.borrow()) / *sx_clone.borrow(),
                (event.offset_y() - *ty_clone.borrow()) / *sy_clone.borrow(),
            ));
        });

        canvas.add_event_listener(move |_event: MouseDownEvent| {
            js! {
                if (window.audioContext.state !== "running" && !window.audioContextResume) {
                    window.audioContext.resume();
                    window.audioContextResume = true;
                    // console.log("AudioContextResume.");
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
            s_key_down
                .borrow_mut()
                .event(Event::KeyPress(event.key().to_uppercase()));
        });
        Ok(())
    }() {
        Ok(_) => (),
        Err(err) => state
            .borrow_mut()
            .handle_error(format!("init error {:?}", err)),
    }
}

pub fn current_timestamp() -> f64 {
    use stdweb::unstable::TryInto;
    js!(return performance.now();).try_into().unwrap()
}

pub fn random() -> f64 {
    use stdweb::unstable::TryInto;
    return js! {return Math.random();}.try_into().unwrap();
}

pub fn log<T: std::fmt::Debug>(s: T) {
    console!(log, format!("{:?}", s));
}

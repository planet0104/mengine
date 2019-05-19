use std::ffi::OsStr;
use std::path::Path;
use std::rc::Rc;
use stdweb::traits::*;
use stdweb::unstable::TryInto;
use stdweb::web::html_element::ImageElement;
use stdweb::web::IElement;
use stdweb::web::{document, CanvasRenderingContext2d};
use png::HasParameters;

use super::{AnimationTimer, Graphics, AudioType, Event, Window, Image, Settings, State, Transform};
use std::any::Any;
use std::cell::RefCell;
use stdweb::web::event::{
    ClickEvent,
    ITouchEvent,
    KeyDownEvent,
    KeyUpEvent,
    MouseDownEvent,
    PointerMoveEvent,
    ResizeEvent,
    TouchMove,
};
use stdweb::web::html_element::CanvasElement;

struct WebImage {
    image: ImageElement,
}
impl Image for WebImage {
    fn width(&self) -> f64 {
        self.image.width() as f64
    }
    fn height(&self) -> f64 {
        self.image.height() as f64
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct BrowserWindow{
    timer: AnimationTimer,
    ups_count: u64,
    fps_count: u64,
    ups: u64,
    fps: u64,
    ups_fps_timer: AnimationTimer
}
impl Window for BrowserWindow{
    fn set_update_rate(&mut self, ups: u64){
        // if !set_worker_update_rate(ups){
        self.timer.set_fps(ups as f64);
        // }
    }

    fn load_image(&mut self, path: &str) -> Result<Rc<Image>, String> {
        let image = ImageElement::new();
        let web_image = Rc::new(WebImage { image });
        web_image.image.set_src(path);
        Ok(web_image)
    }

    fn load_image_alpha(&mut self, image: &image::RgbaImage) -> Result<Rc<Image>, String>{
        let mut png_data:Vec<u8> = vec![];
        {
            let mut encoder = png::Encoder::new(&mut png_data, image.width(), image.height());
            encoder.set(png::ColorType::RGBA);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&image).unwrap();
        }

        let mut png_base64 = base64::encode(&png_data);
        png_base64.insert_str(0, "data:image/png;base64,");
        let image = ImageElement::new();
        let web_image = Rc::new(WebImage { image });
        web_image.image.set_src(&png_base64);
        Ok(web_image)
    }
}

struct BrowserGraphics {
    font_family: String,
    context: CanvasRenderingContext2d,
}

impl BrowserGraphics {
    fn transform(&self, transform: Option<Transform>) {
        self.context.save();
        if let Some(transform) = transform {
            self.context
                .translate(transform.translate.0, transform.translate.1);
            self.context.rotate(transform.rotate);
        }
    }
}

impl Graphics for BrowserGraphics {
    fn clear_rect(&mut self, color: &[u8; 4], x: f64, y: f64, width: f64, height: f64) {
        self.context.set_fill_style_color(&format!(
            "rgba({},{},{},{})",
            color[0], color[1], color[2], color[3]
        ));
        self.context.fill_rect(x, y, width, height);
    }

    fn draw_image(
        &mut self,
        transform: Option<Transform>,
        image: &Image,
        src: Option<[f64; 4]>,
        dest: Option<[f64; 4]>,
    ) -> Result<(), String> {
        match image.as_any().downcast_ref::<WebImage>() {
            Some(image) => {
                self.transform(transform);

                let ret = match if src.is_none() && dest.is_none() {
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
                };
                self.context.restore();
                ret
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
        self.transform(transform);
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
        self.context.restore();
        Ok(())
    }
}

pub fn play_sound(assets: &mut super::AssetsFile, _t: AudioType) {
    if let Some(data) = assets.data() {
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
}

pub fn run<S: State>(title: &str, width: f64, height: f64, settings: Settings) {
    let is_weixin: bool = js! {
        //判断微信
        var ua = navigator.userAgent.toLowerCase();
        if(ua.match(/MicroMessenger/i)=="micromessenger") {
            //微信内置浏览器不用setTimeout和setInterval因为限制为30帧
            return true;
        } else {
            return false;
        }
    }
    .try_into()
    .unwrap();
    document()
        .body()
        .expect("no html body!!")
        .append_html("<canvas id=\"canvas\"></canvas><audio id=\"backgroundAudio\"></audio>")
        .expect("append canvas fail!!");

    let canvas: CanvasElement = document()
        .query_selector("#canvas")
        .unwrap()
        .unwrap()
        .try_into()
        .unwrap();

    let window = Rc::new(RefCell::new(BrowserWindow {
        ups_count: 0,
        fps_count: 0,
        ups: 0,
        fps: 0,
        ups_fps_timer: AnimationTimer::new(1.0),
        timer: AnimationTimer::new(settings.ups as f64),
    }));

    let mut graphics = BrowserGraphics {
        font_family: "Arial".to_string(),
        context: canvas.get_context().unwrap(),
    };

    let state = Rc::new(RefCell::new(S::new(&mut *window.borrow_mut())));
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

        let context = Context {
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
        let webwindow = stdweb::web::window();
        let element = document().query_selector("#canvas")?;
        if element.is_none() {
            state
                .borrow_mut()
                .handle_error("canvas is None!".to_string());
            return Ok(());
        }
        let canvas: CanvasElement = element.unwrap().try_into()?;
        canvas.set_width(webwindow.inner_width() as u32);
        canvas.set_height(webwindow.inner_height() as u32);
        canvas.set_attribute(
            "style",
            &format!(
                "width:{}px;height:{}px",
                webwindow.inner_width(),
                webwindow.inner_height()
            ),
        )?;
        document().set_title(title);

        //随窗口更改canvas大小
        webwindow.add_event_listener(|_event: ResizeEvent| {
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


        // -------------- 更新函数部分 --------------------
        
        
        //先检查是否支持worker
        let mut use_worker = true;
        let s_update = state.clone();
        let winclone = window.clone();
        if !create_update_worker(settings.ups, move ||{
            let mut w = winclone.borrow_mut();

            //{worker中使用setInterval因此不用判断timer}!!!
            //s_update.borrow_mut().update(&mut *w);
            if w.timer.ready_for_next_frame(){
                s_update.borrow_mut().update(&mut *w);
                w.ups_count += 1;
            }

            if w.ups_fps_timer.ready_for_next_frame(){
                w.ups = w.ups_count;
                w.fps = w.fps_count;
                w.ups_count = 0;
                w.fps_count = 0;
            }
        }){
            use_worker = false;
            //不支持worker则使用setTimeout方法 调用 update
            if !is_weixin {
                //重新创建一份callback
                let s_update = state.clone();
                let winclone = window.clone();
                js! {
                    var updatefn = @{
                        //setTimeOut使用最快速度，每次都要判断timer
                        move ||{
                            let mut w = winclone.borrow_mut();

                            if w.timer.ready_for_next_frame(){
                                s_update.borrow_mut().update(&mut *w);
                                w.ups_count += 1;
                            }
                            if w.ups_fps_timer.ready_for_next_frame(){
                                w.ups = w.ups_count;
                                w.fps = w.fps_count;
                                w.ups_count = 0;
                                w.fps_count = 0;
                            }
                        }
                    };
                    var u =  function(){
                        updatefn();
                        setTimeout(u, 1);
                    };
                    setTimeout(u, 1);
                    //setInterval(updatefn, 1);
                    //setTimeout 或 setInterval 频率最高220~230
                };
            }
        }

        let s_update = state.clone();
        // request_animation_frame
        let s_animation = state.clone();
        let (auto_scale, draw_center) = (settings.auto_scale, settings.draw_center);
        let background_color = settings.background_color.unwrap_or([0, 0, 0, 255]);
        let (tx_clone, ty_clone, sx_clone, sy_clone) = (
            trans_x.clone(),
            trans_y.clone(),
            scale_x.clone(),
            scale_y.clone(),
        );

        let winclone = window.clone();
        let mut animation_fn = move |_timestamp| {
            //微信内置浏览器在request_animation_frame中运行以提高更新频率
            if is_weixin && !use_worker {
                //判断timer以支持较低帧率
                if winclone.borrow_mut().timer.ready_for_next_frame() {
                    s_update.borrow_mut().update(&mut *winclone.borrow_mut());
                    winclone.borrow_mut().ups_count += 1;
                }

                let mut w = winclone.borrow_mut();
                if w.ups_fps_timer.ready_for_next_frame(){
                    w.ups = w.ups_count;
                    w.fps = w.fps_count;
                    w.ups_count = 0;
                    w.fps_count = 0;
                }
            }
            winclone.borrow_mut().fps_count += 1;
            let (window_width, window_height) =
                (webwindow.inner_width() as f64, webwindow.inner_height() as f64);
            let mut state = s_animation.borrow_mut();
            graphics.clear_rect(&background_color, 0., 0., window_width, window_height);
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
            let (mut trans_x, mut trans_y) = (0.0, 0.0);
            if draw_center {
                trans_x = (window_width - new_width) / 2.;
                trans_y = (window_height - new_height) / 2.;
                graphics
                    .context
                    .translate(trans_x / scale_x, trans_y / scale_y);
                *tx_clone.borrow_mut() = trans_x;
                *ty_clone.borrow_mut() = trans_y;
            }

            graphics.context.save();
            if let Err(err) = state.draw(&mut graphics) {
                state.handle_error(format!("draw error {:?}", err));
            }
            
            //显示UPS/FPS
            if settings.show_ups_fps{
                let _ = graphics.draw_text(
                None,
                &format!(
                    "UPS/FPS:{}/{}",
                    winclone.borrow().ups,
                    winclone.borrow().fps
                ),
                20.,
                height-30.,
                &[255, 255, 0, 200],
                10);
            }

            graphics.context.restore();
            if draw_center {
                graphics.context.restore();
            }
            //遮盖上部分窗口
            graphics.clear_rect(&background_color, 0., 0., window_width, trans_y);
            //遮盖下部分窗口
            graphics.clear_rect(
                &background_color,
                0.,
                trans_y + height * scale_y,
                window_width,
                window_height - (trans_y + height * scale_y),
            );
            //遮盖左部分窗口
            graphics.clear_rect(&background_color, 0.0, 0.0, trans_x, window_height);
            //遮盖右部分窗口
            graphics.clear_rect(
                &background_color,
                trans_x + width * scale_x,
                0.0,
                window_width - (trans_x + width * scale_x),
                window_height,
            );
        };
        animation_fn(0.0);

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
        let winclone = window.clone();
        canvas.add_event_listener(move |event: PointerMoveEvent| {
            s_mouse_move.borrow_mut().event(Event::MouseMove(
                (event.offset_x() - *tx_clone.borrow()) / *sx_clone.borrow(),
                (event.offset_y() - *ty_clone.borrow()) / *sy_clone.borrow(),
            ), &mut *winclone.borrow_mut());
        });

        let s_touch_move = state.clone();
        let winclone = window.clone();
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
                ), &mut *winclone.borrow_mut());
            }
        });

        let s_click = state.clone();
        let winclone = window.clone();
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
            ), &mut *winclone.borrow_mut());
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
        let s_key_up = state.clone();
        let winclone = window.clone();
        document().add_event_listener(move |event: KeyUpEvent| {
            event.prevent_default();
            s_key_up
                .borrow_mut()
                .event(Event::KeyUp(event.key().to_uppercase()), &mut *winclone.borrow_mut());
        });

        let s_key_down = state.clone();
        let winclone = window.clone();
        document().add_event_listener(move |event: KeyDownEvent| {
            event.prevent_default();
            s_key_down
                .borrow_mut()
                .event(Event::KeyDown(event.key().to_uppercase()), &mut *winclone.borrow_mut());
        });

        Ok(())
    }() {
        Ok(_) => (),
        Err(err) => state
            .borrow_mut()
            .handle_error(format!("init error {:?}", err)),
    }
}

///设置Worker的更新频率
fn set_worker_update_rate(ups: u64) -> bool{
    let ret: bool = js!{
        if(window.update_worker){
            window.update_worker.postMessage(@{ups as i32});
            return true;
        }else{
            return false;
        }
    }.try_into().unwrap_or(false);
    ret
}

///使用worker加速update_rate
fn create_update_worker<F:Fn()+'static>(ups: u64, callback:F) -> bool{
    // let mut worker = String::from(r#"
    //     var ups = _60_;
    //     var interval;

    //     function send(){
    //         postMessage('update');
    //     }

    //     onmessage = function(e) {
    //         ups = e.data;
    //         if(interval){
    //             clearInterval(interval);
    //         }
    //         interval = setInterval(send, 1000.0/ups);
    //     };
    //     interval = setInterval(send, 1000.0/ups);
    // "#);

    let mut worker = String::from(r#"
        function send(){
            postMessage('update');
        }
        var interval = setInterval(send, 1);
    "#);

    worker = worker.replace("_60_", &format!("{}.0", ups));

    let ret:bool = js!{
        var update_callback = @{callback};

        //创建worker
        try{
            var blob;
            try {
                blob = new Blob([@{worker}], {type: "application/javascript"});
            } catch (e) { // Backwards-compatibility
                window.BlobBuilder = window.BlobBuilder || window.WebKitBlobBuilder || window.MozBlobBuilder;
                blob = new BlobBuilder();
                blob.append(response);
                blob = blob.getBlob();
            }
            var worker = new Worker(URL.createObjectURL(blob));
            worker.onmessage = function(msg){
                update_callback();
            };
            window.update_worker = worker;
            return true;
        }catch(e){
            return false;
        }
    }.try_into().unwrap_or(false);
    ret
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

pub fn play_music(file: &str, repeat: bool) {
    js! {
        var url = @{file};
        var repeat = @{repeat};
        var audio = document.getElementById("backgroundAudio");
        if(audio.src == url){
            if(audio.networkState == 1){
                audio.play();
            }
        }else{
            audio.src = url;
            audio.play();
            audio.loop = repeat;
            audio.preload = true;
        }
    };
}

pub fn stop_music() {
    js! {
        var audio = document.getElementById("backgroundAudio");
        audio.pause();
    };
}

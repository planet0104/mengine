use askama::Template;
use png::HasParameters;
use std::io::Result;
use std::path::Path;
use std::rc::Rc;
use std::sync::mpsc::{channel, Sender};
use stdweb::traits::*;
use stdweb::unstable::TryInto;
use stdweb::web::event::ReadyStateChangeEvent;
use stdweb::web::html_element::ImageElement;
use stdweb::web::ArrayBuffer;
use stdweb::web::IElement;
use stdweb::web::IEventTarget;
use stdweb::web::XhrReadyState;
use stdweb::web::XhrResponseType;
use stdweb::web::XmlHttpRequest;
use stdweb::web::{document, CanvasRenderingContext2d};

use super::{
    AnimationTimer, Assets, AssetsType, AudioType, Event, Graphics, Settings, State, Transform,
    Window,
};
use std::cell::RefCell;
use stdweb::web::event::{
    ClickEvent, ITouchEvent, KeyDownEvent, KeyUpEvent, MouseDownEvent, PointerMoveEvent,
    ResizeEvent, TouchMove,
};
use stdweb::web::html_element::CanvasElement;

enum RawAssets {
    Image(ImageElement),
    Blob(Vec<u8>),
    Value(stdweb::Value),
}

#[derive(Debug, Clone)]
pub struct Sound {
    audio_type: AudioType,
    buffer: stdweb::Value,
}

#[derive(Debug, Clone)]
pub struct Image {
    image: ImageElement,
}

impl Image {
    pub fn width(&self) -> f64 {
        self.image.width() as f64
    }
    pub fn height(&self) -> f64 {
        self.image.height() as f64
    }
}

struct BrowserWindow {
    timer: AnimationTimer,
    ups_count: u64,
    fps_count: u64,
    ups: u64,
    fps: u64,
    ups_fps_timer: AnimationTimer,
    sender: Sender<(String, AssetsType, Result<RawAssets>)>,
}
impl Window for BrowserWindow {
    fn set_update_rate(&mut self, ups: u64) {
        self.timer.set_fps(ups as f64);
    }

    fn load_assets(&mut self, assets: Vec<(&str, AssetsType)>) {
        let assets: Vec<(String, AssetsType)> = assets
            .iter()
            .map(|(path, tp)| (path.to_string(), *tp))
            .collect();
        for (path, t) in assets {
            let sender = self.sender.clone();
            match t {
                AssetsType::Image => {
                    let onload = move |image: stdweb::Value| {
                        let image: ImageElement = image.try_into().unwrap();
                        let _ = sender.send((
                            image.get_attribute("key").unwrap(),
                            AssetsType::Image,
                            Ok(RawAssets::Image(image)),
                        ));
                    };
                    js! {
                        var image = new Image();
                        var path = @{path};
                        image.src = path;
                        image.setAttribute("key", path);
                        image.onload = function(){
                            @{onload}(this);
                        };
                    };
                }
                AssetsType::Sound | AssetsType::File => {
                    let req = XmlHttpRequest::new();
                    let mpath = path.to_string();
                    req.add_event_listener(move |event: ReadyStateChangeEvent| {
                        //取出XmlHttpRequest和对应的url
                        let req: XmlHttpRequest = js! {return @{event}.target}.try_into().unwrap();
                        let path: String = js!(return @{&mpath}).try_into().unwrap();

                        if req.ready_state() == XhrReadyState::Done {
                            if req.status() == 200 {
                                let array_buffer: ArrayBuffer =
                                    req.raw_response().try_into().unwrap();
                                let contents: Vec<u8> = Vec::from(array_buffer);
                                match t {
                                    AssetsType::File => {
                                        let _ = sender.send((
                                            path,
                                            AssetsType::File,
                                            Ok(RawAssets::Blob(contents)),
                                        ));
                                    }
                                    _ => {
                                        let msender = sender.clone();
                                        let decode_callback =
                                            move |path: stdweb::Value, buffer: stdweb::Value| {
                                                let path: String = path.try_into().unwrap();
                                                let _ = msender.send((
                                                    path,
                                                    AssetsType::Sound,
                                                    Ok(RawAssets::Value(buffer)),
                                                ));
                                            };
                                        js! {
                                            var path = @{path};
                                            var bytes = new Uint8Array(@{contents}).buffer;
                                            var audioCtx = window.audioContext;
                                            audioCtx.decodeAudioData(bytes, function(buffer){
                                                @{decode_callback}(path, buffer);
                                            });
                                        };
                                    }
                                }
                            }
                        }
                    });
                    match req.open("GET", &path) {
                        Ok(_) => (),
                        Err(err) => super::log(format!("{:?}", err)),
                    };
                    if let Err(err) = req.set_response_type(XhrResponseType::ArrayBuffer) {
                        super::log(format!("{:?}", err));
                    }
                    match req.send() {
                        Ok(_) => (),
                        Err(err) => super::log(format!("{:?}", err)),
                    };
                }
            }
        }
    }

    fn load_image(&mut self, width: u32, height: u32, key: &str, data: Vec<u8>) {
        let mut png_data: Vec<u8> = vec![];
        {
            let mut encoder = png::Encoder::new(&mut png_data, width, height);
            encoder.set(png::ColorType::RGBA);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&data).unwrap();
        }

        let mut png_base64 = base64::encode(&png_data);
        png_base64.insert_str(0, "data:image/png;base64,");
        let image = ImageElement::new();
        image.set_src(&png_base64);
        let _ = self.sender.send((
            String::from(key),
            AssetsType::Image,
            Ok(RawAssets::Image(image)),
        ));
    }

    fn load_svg(&mut self, key: &str, svg: String) {
        let mut svg_base64 = base64::encode(&svg);
        svg_base64.insert_str(0, "data:image/svg+xml;base64,");
        let image = ImageElement::new();
        image.set_src(&svg_base64);
        let _ = self.sender.send((
            String::from(key),
            AssetsType::Image,
            Ok(RawAssets::Image(image)),
        ));
    }
}

struct BrowserGraphics {
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
    fn fill_rect(&mut self, color: &[u8; 4], x: f64, y: f64, width: f64, height: f64) {
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
    ) {
        self.transform(transform);

        let _ = match if src.is_none() && dest.is_none() {
            self.context.draw_image(image.image.clone(), 0., 0.)
        } else if src.is_none() && dest.is_some() {
            let dest = dest.unwrap();
            self.context
                .draw_image_d(image.image.clone(), dest[0], dest[1], dest[2], dest[3])
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
    }

    fn draw_text(&mut self, cotnent: &str, x: f64, y: f64, color: &[u8; 4], font_size: u32) {
        self.context.set_fill_style_color(&format!(
            "rgba({},{},{},{})",
            color[0],
            color[1],
            color[2],
            color[3] as f64 / 255.0
        ));
        let font = format!("{}px sans-serif", font_size);
        if self.context.get_font() != font {
            self.context.set_font(&font);
        }
        self.context
            .set_text_baseline(stdweb::web::TextBaseline::Top);
        self.context.fill_text(cotnent, x, y, None);
    }
}

pub fn play_sound(sound: &Sound) {
    js! {
        try{
                var audioCtx = window.audioContext;
                var source = audioCtx.createBufferSource();
                source.buffer = @{sound.buffer.clone()};
                source.connect(audioCtx.destination);
                source.start(0);
            }catch(e){
                console.log(e);
        }
    };
}

fn is_weixin() -> bool {
    js! (
        //判断微信
        var ua = navigator.userAgent.toLowerCase();
        if(ua.match(/MicroMessenger/i)=="micromessenger") {
            //微信内置浏览器不用setTimeout和setInterval因为限制为30帧
            return true;
        } else {
            return false;
        }
    )
    .try_into()
    .unwrap()
}

#[derive(Template)]
#[template(path = "head_part.html")]
struct HeaderPart<'a> {
    icon_type: &'a str,
    icon_href: &'a str,
}

#[derive(Template)]
#[template(path = "body_part.html")]
struct BodyPart {}

pub fn run<S: State>(title: &str, width: f64, height: f64, settings: Settings) {
    stdweb::initialize();
    let body = BodyPart {};
    let _ = document()
        .body()
        .unwrap()
        .append_html(&body.render().unwrap());

    let canvas: CanvasElement = document()
        .query_selector("#canvas")
        .unwrap()
        .unwrap()
        .try_into()
        .unwrap();

    let (sender, receiver) = channel();

    let game_window = Rc::new(RefCell::new(BrowserWindow {
        sender,
        ups_count: 0,
        fps_count: 0,
        ups: 0,
        fps: 0,
        ups_fps_timer: AnimationTimer::new(1.0),
        timer: AnimationTimer::new(settings.ups as f64),
    }));

    let context: CanvasRenderingContext2d = canvas.get_context().unwrap();
    let mut graphics = BrowserGraphics { context: context };

    let game_state = Rc::new(RefCell::new(S::new(&mut *game_window.borrow_mut())));
    let (trans_x, trans_y) = (Rc::new(RefCell::new(0.0)), Rc::new(RefCell::new(0.0)));
    let (scale_x, scale_y) = (Rc::new(RefCell::new(1.0)), Rc::new(RefCell::new(1.0)));

    //声音播放
    js! {
        const AudioContext = window.AudioContext || window.webkitAudioContext;
        window.audioContext = new AudioContext();
        window.audioContextResume = false;
    };

    //---------- 添加head ---------------

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

    let header = HeaderPart {
        icon_type: icon_type,
        icon_href: icon_path,
    };
    let _ = document()
        .head()
        .unwrap()
        .append_html(&header.render().unwrap());

    let window = stdweb::web::window();

    canvas.set_width(window.inner_width() as u32);
    canvas.set_height(window.inner_height() as u32);
    canvas
        .set_attribute(
            "style",
            &format!(
                "width:{}px;height:{}px;",
                window.inner_width(),
                window.inner_height()
            ),
        )
        .unwrap();
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

    // -------------- 更新函数部分 --------------------

    let s_update = game_state.clone();
    let gw = game_window.clone();

    let callback = move || {
        let mut w = gw.borrow_mut();

        if w.timer.ready_for_next_frame() {
            s_update.borrow_mut().update(&mut *w);
            w.ups_count += 1;
            if let Ok((path, t, result)) = receiver.try_recv() {
                match result {
                    Ok(RawAssets::Image(image)) => s_update.borrow_mut().on_assets_load(
                        &path,
                        AssetsType::Image,
                        Ok(Assets::Image(Image { image })),
                        &mut *w,
                    ),
                    Ok(RawAssets::Value(data)) => {
                        s_update.borrow_mut().on_assets_load(
                            &path,
                            t,
                            Ok(Assets::Sound(Sound {
                                buffer: data,
                                audio_type: AudioType::test(&path),
                            })),
                            &mut *w,
                        );
                    }
                    Ok(RawAssets::Blob(data)) => {
                        s_update.borrow_mut().on_assets_load(
                            &path,
                            t,
                            Ok(Assets::File(data)),
                            &mut *w,
                        );
                    }
                    Err(err) => {
                        s_update
                            .borrow_mut()
                            .on_assets_load(&path, t, Err(err), &mut *w);
                    }
                };
            }
        }

        if w.ups_fps_timer.ready_for_next_frame() {
            w.ups = w.ups_count;
            w.fps = w.fps_count;
            w.ups_count = 0;
            w.fps_count = 0;
        }
    };

    start_update_loop(callback);

    let (auto_scale, draw_center) = (settings.auto_scale, settings.draw_center);
    let background_color = settings.background_color.unwrap_or([0, 0, 0, 255]);
    let (tx_clone, ty_clone, sx_clone, sy_clone) = (
        trans_x.clone(),
        trans_y.clone(),
        scale_x.clone(),
        scale_y.clone(),
    );

    let winclone = game_window.clone();
    let state_anim = game_state.clone();
    let mut animation_fn = move |_timestamp| {
        winclone.borrow_mut().fps_count += 1;
        let (window_width, window_height) =
            (window.inner_width() as f64, window.inner_height() as f64);
        let mut state = state_anim.borrow_mut();
        graphics.fill_rect(&background_color, 0., 0., window_width, window_height);
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
        state.draw(&mut graphics, &mut *winclone.borrow_mut());

        //显示UPS/FPS
        if settings.show_ups_fps {
            let _ = graphics.draw_text(
                &format!(
                    "UPS/FPS:{}/{}",
                    winclone.borrow().ups,
                    winclone.borrow().fps
                ),
                20.,
                height - 30.,
                &[255, 255, 0, 200],
                10,
            );
        }

        graphics.context.restore();
        if draw_center {
            graphics.context.restore();
        }
        //遮盖上部分窗口
        graphics.fill_rect(&background_color, 0., 0., window_width, trans_y);
        //遮盖下部分窗口
        graphics.fill_rect(
            &background_color,
            0.,
            trans_y + height * scale_y,
            window_width,
            window_height - (trans_y + height * scale_y),
        );
        //遮盖左部分窗口
        graphics.fill_rect(&background_color, 0.0, 0.0, trans_x, window_height);
        //遮盖右部分窗口
        graphics.fill_rect(
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

    let s_mouse_move = game_state.clone();
    let (tx_clone, ty_clone, sx_clone, sy_clone) = (
        trans_x.clone(),
        trans_y.clone(),
        scale_x.clone(),
        scale_y.clone(),
    );
    let winclone = game_window.clone();
    canvas.add_event_listener(move |event: PointerMoveEvent| {
        s_mouse_move.borrow_mut().event(
            Event::MouseMove(
                (event.offset_x() - *tx_clone.borrow()) / *sx_clone.borrow(),
                (event.offset_y() - *ty_clone.borrow()) / *sy_clone.borrow(),
            ),
            &mut *winclone.borrow_mut(),
        );
    });

    let s_touch_move = game_state.clone();
    let winclone = game_window.clone();
    let (tx_clone, ty_clone, sx_clone, sy_clone) = (
        trans_x.clone(),
        trans_y.clone(),
        scale_x.clone(),
        scale_y.clone(),
    );
    canvas.add_event_listener(move |event: TouchMove| {
        let touchs = event.target_touches();
        if touchs.len() > 0 {
            s_touch_move.borrow_mut().event(
                Event::MouseMove(
                    (touchs[0].client_x() - *tx_clone.borrow()) / *sx_clone.borrow(),
                    (touchs[0].client_y() - *ty_clone.borrow()) / *sy_clone.borrow(),
                ),
                &mut *winclone.borrow_mut(),
            );
        }
    });

    let s_click = game_state.clone();
    let winclone = game_window.clone();
    let (tx_clone, ty_clone, sx_clone, sy_clone) = (
        trans_x.clone(),
        trans_y.clone(),
        scale_x.clone(),
        scale_y.clone(),
    );
    canvas.add_event_listener(move |event: ClickEvent| {
        s_click.borrow_mut().event(
            Event::Click(
                (event.offset_x() - *tx_clone.borrow()) / *sx_clone.borrow(),
                (event.offset_y() - *ty_clone.borrow()) / *sy_clone.borrow(),
            ),
            &mut *winclone.borrow_mut(),
        );
    });

    canvas.add_event_listener(move |_event: MouseDownEvent| {
        js! {
            if (window.audioContext.state !== "running" && !window.audioContextResume) {
                window.audioContext.resume();
                window.audioContextResume = true;
            }
        };
    });
    // document().add_event_listener(move |event: KeyPressEvent| {
    //     event.prevent_default();
    //     STATE.with(|state|{ state.borrow_mut().as_mut().unwrap().event(Event::KeyPress(event.key().to_uppercase())); });
    // });
    let s_key_up = game_state.clone();
    let winclone = game_window.clone();
    document().add_event_listener(move |event: KeyUpEvent| {
        event.prevent_default();
        s_key_up.borrow_mut().event(
            Event::KeyUp(event.key().to_uppercase()),
            &mut *winclone.borrow_mut(),
        );
    });

    let s_key_down = game_state.clone();
    let winclone = game_window.clone();
    document().add_event_listener(move |event: KeyDownEvent| {
        event.prevent_default();
        s_key_down.borrow_mut().event(
            Event::KeyDown(event.key().to_uppercase()),
            &mut *winclone.borrow_mut(),
        );
    });
}

/// 在worker中使用interval加速callback调用频率(目前限制最快1000ups)
///
/// 创建worker失败则使用setTimeout调用callback(最快230ups)
///
/// 如果是微信，不用timerout直接返回callback，请在request_animation_frame调用callback(60ups)
fn start_update_loop<F: Fn() + 'static>(callback: F) {
    let worker = String::from(
        r#"
        function send(){
            postMessage('update');
        }
        var interval = setInterval(send, 1);
    "#,
    );

    // 创建worker
    if js!{
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
            window.update_worker = worker;
            return true;
        }catch(e){
            return false;
        }
    }.try_into().unwrap_or(false){
        //启动worker
        js!{
            var update_callback = @{callback};
            window.update_worker.onmessage = function(msg){
                update_callback();
            };
        };
    }else{
        if is_weixin(){
            js!{
                //微信内置浏览器在request_animation_frame中运行以提高更新频率
                var oldRequestAnimationFrame = window.requestAnimationFrame;
                window.requestAnimationFrame = function(a, b){
                    @{callback}();
                    oldRequestAnimationFrame(a, b);
                };
            };
        }else{
            //worker创建失败,使用setTimeout
            js!{
                var u =  function(){
                    @{callback}();
                    setTimeout(u, 1);
                };
                setTimeout(u, 1);
            };
        }
    }
}

pub fn current_timestamp() -> f64 {
    js!(return performance.now();).try_into().unwrap()
}

pub fn random() -> f64 {
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

pub fn alert(_head: &str, msg: &str) {
    js! {
        alert(@{msg});
    };
}

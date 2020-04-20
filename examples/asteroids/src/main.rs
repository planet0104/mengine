use std::io::Result;
use std::collections::HashMap;

use mengine::*;

const ASSET_ASTEROID: &str = "asteroid.png";
const ASSET_MOON: &str = "moon.png";

const RESOURCES: &'static [(&'static str, AssetsType); 2] = &[
    (ASSET_ASTEROID, AssetsType::Image),
    (ASSET_MOON, AssetsType::Image)
];

struct Asteroid {
    anim: Animation,
    region: [f64; 4],
}

impl Asteroid {
    fn draw(&mut self, g: &mut impl Graphics) {
        self.anim.draw(None, g, self.region);
    }
    fn update(&mut self) {
        self.anim.update();
    }
}

struct Moon{
    image: Image,
    rotation: f64
}

impl Moon{
    fn draw(&mut self, g: &mut impl Graphics) {
        let (r1, r2) = (self.image.width()/2., self.image.height()/2.);
        g.draw_image_at(
            Some(Transform {
                rotate: self.rotation,
                translate: (190., 190.)
            }),
            &self.image,
            -r1,
            -r2,
        );
    }
    fn update(&mut self) {
        self.rotation += 0.001;
    }
}

struct Game {
    resources: HashMap<String, Assets>,
    asteroid: Option<Asteroid>,
    moon: Option<Moon>
}

impl Game{
    fn start(&mut self){
        for (path, image) in &self.resources{
            //creat Asteroid
            if path == ASSET_ASTEROID{
                let mut frames = vec![];
                for y in (0..1148).step_by(82) {
                    frames.push([0., y as f64, 99., 82.]);
                }
                let mut anim = Animation::active(image.as_image().unwrap(), frames, 15.0);
                anim.set_repeat(true);
                self.asteroid = Some(Asteroid {
                    anim,
                    region: [20., 40., 99., 82.],
                });
            }
            //create Moon
            if path == ASSET_MOON{
                self.moon = Some(
                    Moon{
                        image: image.as_image().unwrap(),
                        rotation: 0.0
                    }
                );
            }
        }
    }
}

impl State for Game {
    fn new(window: &mut impl Window) -> Self {
        window.load_assets(RESOURCES);
        Game { resources: HashMap::new(), asteroid:None, moon: None }
    }

    fn on_assets_load(
        &mut self,
        path: &str,
        _: AssetsType,
        assets: Result<Assets>,
        _window: &mut impl Window,
    ) {
        match assets {
            Ok(assets) => {
                self.resources.insert(path.to_string(), assets);
            }
            Err(err) => {
                println!("图片加载失败！{:?}", err);
            }
        }

        if self.resources.len() == RESOURCES.len(){
            self.start();
        }
    }

    fn event(&mut self, event: Event, window: &mut impl Window) {
        match event {
            Event::Click(x, y) => {
                log(format!("Click: {}x{}", x, y));
            }
            Event::KeyUp(key) => {
                println!("key={}", key);
                match key.as_str() {
                    "F1" => window.set_update_rate(60),
                    "F2" => window.set_update_rate(600),
                    _ => (),
                };
            }
            _ => (),
        }
    }

    fn draw(&mut self, g: &mut impl Graphics, _window: &mut impl Window) {
        g.fill_rect(&[0, 0, 0, 255], 0., 0., 300., 300.);
        if let Some(asteroid) = &mut self.asteroid {
            asteroid.draw(g);
            self.moon.as_mut().unwrap().draw(g);
            g.draw_text("Asteroid", 5., 5., &[255, 255, 255, 255], 20);
        } else {
            g.draw_text("Loading...", 100., 110., &[255, 255, 255, 255], 30);
        }
    }

    fn update(&mut self, _window: &mut impl Window) {
        if let Some(asteroid) = &mut self.asteroid {
            asteroid.update();
            self.moon.as_mut().unwrap().update();
        }
    }
}

fn main() {
    mengine::run::<Game>(
        "Asteroid",
        300.,
        300.,
        Settings {
            show_ups_fps: true,
            background_color: Some([255, 255, 255, 255]),
            draw_center: true,
            ..Default::default()
        },
    );
}

use mengine::*;
use std::io::Result;

struct Asteroid {
    anim: Animation,
    region: [f64; 4],
}

impl Asteroid {
    fn draw(&mut self, g: &mut Graphics) {
        self.anim.draw(None, g, self.region);
    }
    fn update(&mut self) {
        self.anim.update();
    }
}

struct Game {
    sprite: Option<Asteroid>,
}

impl State for Game {
    fn new(window: &mut Window) -> Self {
        window.load_assets(vec![("asteroid.png", AssetsType::Image)]);
        Game { sprite: None }
    }

    fn on_assets_load(
        &mut self,
        _path: &str,
        _: AssetsType,
        assets: Result<Assets>,
        _window: &mut Window,
    ) {
        match assets {
            Ok(Assets::Image(image)) => {
                let mut frames = vec![];
                for y in (0..1148).step_by(82) {
                    frames.push([0., y as f64, 99., 82.]);
                }
                let mut anim = Animation::active(image, frames, 15.0);
                anim.set_repeat(true);
                self.sprite = Some(Asteroid {
                    anim,
                    region: [100., 100., 99., 82.],
                });
            }
            Err(err) => {
                println!("图片加载失败！{:?}", err);
            }
            _ => (),
        }
    }

    fn event(&mut self, event: Event, window: &mut Window) {
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

    fn draw(&mut self, g: &mut Graphics, _window: &mut Window) {
        g.fill_rect(&[0, 0, 0, 255], 0., 0., 300., 300.);
        if let Some(sprite) = &mut self.sprite {
            sprite.draw(g);
            g.draw_text("Asteroid", 5., 5., &[255, 255, 255, 255], 20);
        } else {
            g.draw_text("Loading...", 100., 110., &[255, 255, 255, 255], 30);
        }
    }

    fn update(&mut self, _window: &mut Window) {
        if let Some(sprite) = &mut self.sprite {
            sprite.update();
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

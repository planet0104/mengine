use mengine::*;

struct Asteroid {
    anim: Animation,
    region: Rect<f64>,
}

impl Asteroid {
    fn draw(&mut self, g: &mut Graphics) -> Result<(), String> {
        self.anim.draw(g, &self.region)
    }
    fn update(&mut self) {
        self.anim.update();
    }
}

struct Game {
    sprites: Vec<Asteroid>,
}

impl State for Game {
    fn on_load(&mut self, image_loader: &mut ImageLoader) {
        let images = image_loader.load("asteroid.png").unwrap();
        let mut frames = vec![];
        for y in (0..1148).step_by(82) {
            frames.push(Rect::new(0., y as f64, 99., 82.));
        }
        self.sprites.push(Asteroid {
            anim: Animation::new(images, frames, 5),
            region: Rect::new(100., 100., 99., 82.),
        });
    }

    fn draw(&mut self, g: &mut Graphics) -> Result<(), String> {
        g.clear_rect(&[0, 0, 0, 255], 0., 0., 300., 300.);
        g.draw_text("Asteroid", 5., 20., &[255, 255, 255, 255], 10)?;
        self.sprites[0].draw(g)
    }

    fn update(&mut self) {
        self.sprites[0].update();
    }
}

fn main() {
    let game = Game { sprites: vec![] };
    mengine::run(
        "Asteroid",
        300.,
        300.,
        Settings {
            font_file: Some("FZFSJW.TTF"),
            ..Default::default()
        },
        game,
    );
}

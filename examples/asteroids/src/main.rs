use mengine::*;

struct Asteroid {
    anim: Animation,
    region: [f64; 4],
}

impl Asteroid {
    fn draw(&mut self, g: &mut Graphics) -> Result<(), String> {
        self.anim.draw(g, self.region)
    }
    fn update(&mut self) {
        self.anim.update();
    }
}

struct Game {
    sprite: Asteroid,
}

impl State for Game {
    fn new(image_loader: &mut ImageLoader) -> Self{
        let images = image_loader.load("asteroid.png").unwrap();
        let mut frames = vec![];
        for y in (0..1148).step_by(82) {
            frames.push([0., y as f64, 99., 82.]);
        }

        Game{
            sprite: Asteroid {
                anim: Animation::active(images, frames, 5),
                region: [100., 100., 99., 82.],
            }
        }
    }
    
    fn event(&mut self, event: Event){
        match event{
            Event::Click(x, y) => {
                log(format!("Click: {}x{}", x, y));
            }
            _ => ()
        }
    }

    fn draw(&mut self, g: &mut Graphics) -> Result<(), String> {
        g.clear_rect(&[0, 0, 0, 255], 0., 0., 300., 300.);
        g.draw_text("Asteroid", 5., 20., &[255, 255, 255, 255], 10)?;
        self.sprite.draw(g)
    }

    fn update(&mut self) {
        self.sprite.update();
    }
}

fn main() {
    mengine::run::<Game>(
        "Asteroid",
        300.,
        300.,
        Settings {
            font_file: Some("FZFSJW.TTF"),
            ..Default::default()
        }
    );
}

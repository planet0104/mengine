use mengine::{State, Animation, ImageLoader, Graphics};

struct Game{
    asteroid: Option<Animation>,
}

impl State for Game{
    fn on_load(&mut self, image_loader:&mut ImageLoader){
        let images = image_loader.load("asteroid.png").unwrap();
        let mut frames = vec![];
        for y in (0..1148).step_by(82){
            frames.push([0., y as f64, 99., 82.]);
        }
        self.asteroid = Some(Animation::new(images, frames, 5));
    }

    fn draw(&mut self, g:&mut Graphics) -> Result<(), String>{
        g.clear_rect(&[0, 0, 0, 255], 0., 0., 300., 300.);
        self.asteroid.as_ref().unwrap().draw(g, [100., 100., 99., 82.])?;
        g.draw_text("Asteroid", 5., 20., &[255, 255, 255, 255], 10)?;
        Ok(())
    }

    fn update(&mut self){
        self.asteroid.as_mut().unwrap().update();
    }
}

fn main() {
    mengine::run("Asteroid", 300., 300., Some(60), Game{asteroid: None}, Some("FZFSJW.TTF"));
}
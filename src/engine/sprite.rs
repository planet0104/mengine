use crate::*;
use std::cmp;

//Sprite主要代码

pub type SPRITEACTION = u32;
pub const SA_NONE: SPRITEACTION = 0;
pub const SA_KILL: SPRITEACTION = 1;
pub const SA_ADDSPRITE: SPRITEACTION = 2;

pub type BOUNDSACTION = u32;
pub const BA_STOP: BOUNDSACTION = 0;
pub const BA_WRAP: BOUNDSACTION = 1;
pub const BA_BOUNCE: BOUNDSACTION = 2;
pub const BA_DIE: BOUNDSACTION = 3;

pub trait SpriteExt {
    fn add_sprite(&self, sprite: &Sprite) -> Sprite;
    fn update(&self, sprite_action: SPRITEACTION) -> SPRITEACTION;
}

pub enum Resource {
    Static(Image),
    Animation(Animation),
}

impl Resource {
    pub fn height(&self) -> f64 {
        match &self {
            &Resource::Animation(anim) => anim.frame_height(),
            &Resource::Static(image) => image.height(),
        }
    }

    pub fn width(&self) -> f64 {
        match &self {
            &Resource::Animation(anim) => anim.frame_width(),
            &Resource::Static(image) => image.width(),
        }
    }
}

pub struct Sprite {
    id: f64,
    name: String,
    sprite_ext: Option<Box<SpriteExt>>,
    resource: Resource,
    position: Rect,
    bounds: Rect,
    velocity: Point,
    z_order: i32,
    collision: Rect,
    bounds_action: BOUNDSACTION,
    hidden: bool,
    dying: bool,
}

impl Sprite {
    pub fn new(
        name: String,
        resource: Resource,
        position: Point,
        velocity: Point,
        z_order: i32,
        bounds: Rect,
        bounds_action: BOUNDSACTION,
    ) -> Sprite {
        let mut sprite = Sprite {
            id: 0.0,
            name,
            sprite_ext: None,
            position: Rect::new(
                position.x,
                position.y,
                position.x + resource.width(),
                position.y + resource.height(),
            ),
            resource,
            velocity: velocity,
            z_order: z_order,
            bounds: bounds,
            bounds_action: bounds_action,
            hidden: false,
            dying: false,
            collision: Rect::zero(),
        };
        sprite.id = current_timestamp() + random();
        sprite.calc_collision_rect();
        sprite
    }

    pub fn from_bitmap(name: String, resource: Resource, bounds: Rect) -> Sprite {
        Sprite::new(
            name,
            resource,
            Point { x: 0.0, y: 0.0 },
            Point { x: 0.0, y: 0.0 },
            0,
            bounds,
            BA_STOP,
        )
    }

    pub fn with_bounds_action(
        name: String,
        resource: Resource,
        bounds: Rect,
        bounds_action: BOUNDSACTION,
    ) -> Sprite {
        //计算随即位置
        let x_pos = rand_int(0, (bounds.right - bounds.left) as i32);
        let y_pos = rand_int(0, (bounds.bottom - bounds.top) as i32);
        Sprite::new(
            name,
            resource,
            Point {
                x: x_pos as f64,
                y: y_pos as f64,
            },
            Point { x: 0.0, y: 0.0 },
            0,
            bounds,
            bounds_action,
        )
    }

    fn calc_collision_rect(&mut self) {
        let x_shrink = (self.position.left - self.position.right) / 12.0;
        let y_shrink = (self.position.top - self.position.bottom) / 12.0;
        self.collision = self.position;
        self.collision.inflate(x_shrink, y_shrink);
    }

    //-----------------------------------------------------------------
    // Sprite General Methods
    //-----------------------------------------------------------------
    pub fn sprite_update(&mut self) -> SPRITEACTION {
        // See if the sprite needs to be killed
        if self.dying {
            return SA_KILL;
        }

        // Update the frame
        if let Resource::Animation(anim) = &mut self.resource {
            // If it's a one-cycle frame animation, kill the sprite
            let _ = anim.update();
            if !anim.is_repeat() && anim.is_end() {
                self.dying = true;
            }
        }

        // Update the position
        let mut new_position = Point { x: 0.0, y: 0.0 };
        let mut sprite_size = Point { x: 0.0, y: 0.0 };
        let mut bounds_size = Point { x: 0.0, y: 0.0 };
        new_position.x = self.position.left + self.velocity.x;
        new_position.y = self.position.top + self.velocity.y;
        sprite_size.x = self.position.right - self.position.left;
        sprite_size.y = self.position.bottom - self.position.top;
        bounds_size.x = self.bounds.right - self.bounds.left;
        bounds_size.y = self.bounds.bottom - self.bounds.top;

        // Check the bounds
        // Wrap?
        if self.bounds_action == BA_WRAP {
            if (new_position.x + sprite_size.x) < self.bounds.left {
                new_position.x = self.bounds.right;
            } else if new_position.x > self.bounds.right {
                new_position.x = self.bounds.left - sprite_size.x;
            }
            if (new_position.y + sprite_size.y) < self.bounds.top {
                new_position.y = self.bounds.bottom;
            } else if new_position.y > self.bounds.bottom {
                new_position.y = self.bounds.top - sprite_size.y;
            }
        }
        // Bounce?
        else if self.bounds_action == BA_BOUNCE {
            let mut bounce = false;
            let mut new_velocity = self.velocity;
            if new_position.x < self.bounds.left {
                bounce = true;
                new_position.x = self.bounds.left;
                new_velocity.x = -new_velocity.x;
            } else if (new_position.x + sprite_size.x) > self.bounds.right {
                bounce = true;
                new_position.x = self.bounds.right - sprite_size.x;
                new_velocity.x = -new_velocity.x;
            }
            if new_position.y < self.bounds.top {
                bounce = true;
                new_position.y = self.bounds.top;
                new_velocity.y = -new_velocity.y;
            } else if (new_position.y + sprite_size.y) > self.bounds.bottom {
                bounce = true;
                new_position.y = self.bounds.bottom - sprite_size.y;
                new_velocity.y = -new_velocity.y;
            }
            if bounce {
                self.velocity = new_velocity;
            }
        }
        // Die?
        else if self.bounds_action == BA_DIE {
            if (new_position.x + sprite_size.x) < self.bounds.left
                || new_position.x > self.bounds.right
                || (new_position.y + sprite_size.y) < self.bounds.top
                || new_position.y > self.bounds.bottom
            {
                return SA_KILL;
            }
        }
        // Stop (default)
        else {
            if new_position.x < self.bounds.left
                || new_position.x > (self.bounds.right - sprite_size.x)
            {
                new_position.x = cmp::max(
                    self.bounds.left as i32,
                    cmp::min(
                        new_position.x as i32,
                        self.bounds.right as i32 - sprite_size.x as i32,
                    ),
                ) as f64;
                self.set_velocity(0.0, 0.0);
            }
            if new_position.y < self.bounds.top
                || new_position.y > (self.bounds.bottom - sprite_size.y)
            {
                new_position.y = cmp::max(
                    self.bounds.top as i32,
                    cmp::min(
                        new_position.y as i32,
                        self.bounds.bottom as i32 - sprite_size.y as i32,
                    ),
                ) as f64;
                self.set_velocity(0.0, 0.0);
            }
        }
        self.set_position_point(&new_position);

        SA_NONE
    }

    pub fn update(&mut self) -> SPRITEACTION {
        let sprite_action = self.sprite_update();
        match self.sprite_ext.as_ref() {
            Some(ext) => ext.update(sprite_action),
            _ => sprite_action,
        }
    }

    pub fn draw(&self, g: &mut Graphics) {
        // Draw the sprite if it isn't hidden
        if !self.hidden {
            // Draw the appropriate frame, if necessary
            let dest = [
                self.position.left as f64,
                self.position.top as f64,
                self.resource.width(),
                self.resource.height(),
            ];
            match &self.resource {
                Resource::Animation(anim) => anim.draw(None, g, dest),
                Resource::Static(image) => g.draw_image(None, &image, None, Some(dest)),
            };
        }
    }

    pub fn set_velocity(&mut self, x: f64, y: f64) {
        self.velocity.x = x;
        self.velocity.y = y;
    }

    pub fn set_velocity_point(&mut self, velocity: &Point) {
        self.velocity.x = velocity.x;
        self.velocity.y = velocity.y;
    }

    pub fn velocity(&self) -> &Point {
        &self.velocity
    }

    pub fn set_position_point(&mut self, position: &Point) {
        let dx = position.x - self.position.left;
        let dy = position.y - self.position.top;
        self.position.offset(dx, dy);
        self.calc_collision_rect();
    }

    pub fn set_position(&mut self, x: f64, y: f64) {
        let x = x - self.position.left;
        let y = y - self.position.top;
        self.position.offset(x, y);
        self.calc_collision_rect();
    }

    pub fn set_position_rect(&mut self, position: Rect) {
        self.position = position;
    }

    pub fn test_collison(&self, test: &Rect) -> bool {
        self.collision.left <= test.right
            && test.left <= self.collision.right
            && self.collision.top <= test.bottom
            && test.top <= self.collision.bottom
    }

    pub fn is_point_inside(&self, x: f64, y: f64) -> bool {
        self.position.contain(x, y)
    }

    pub fn height(&self) -> f64 {
        self.resource.height()
    }

    pub fn width(&self) -> f64 {
        self.resource.width()
    }

    pub fn z_order(&self) -> i32 {
        self.z_order
    }

    pub fn resource(&self) -> &Resource {
        &self.resource
    }

    pub fn position(&self) -> &Rect {
        &self.position
    }

    pub fn hidden(&self) -> bool {
        self.hidden
    }

    pub fn ext<T: SpriteExt + 'static>(&mut self, sprite_ext: T) {
        self.sprite_ext = Some(Box::new(sprite_ext));
    }

    pub fn add_sprite(&self) -> Option<Sprite> {
        match self.sprite_ext.as_ref() {
            Some(ext) => Some(ext.add_sprite(self)),
            _ => None,
        }
    }

    pub fn id(&self) -> f64 {
        self.id
    }

    // pub fn set_num_frames(&mut self, num_frames:i32, one_cycle:bool){
    //     self.num_frames = num_frames;
    //     self.one_cycle = one_cycle;

    //     //重新计算位置
    //     self.position.bottom = self.position.top +
    //         (self.position.bottom - self.position.top)/self.num_frames;
    // }

    pub fn kill(&mut self) {
        self.dying = true;
    }

    pub fn dying(&self) -> bool {
        self.dying
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

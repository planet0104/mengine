use super::sprite::{Sprite, SA_ADDSPRITE, SA_KILL};
use crate::*;

//GameEngine 负责创建游戏窗口、绘制和更新精灵
pub trait GameEngine {
    fn sprites_mut(&mut self) -> &mut Vec<Sprite>;
    fn sprites(&mut self) -> &Vec<Sprite>;
    fn sprite_dying(&mut self, sprite_dying_id: usize);
    fn sprite_collision(&mut self, sprite_hitter_id: usize, sprite_hittee_id: usize) -> bool;

    fn add_sprite(&mut self, sprite: Sprite) {
        let sprites = self.sprites_mut();
        if sprites.len() > 0 {
            for i in 0..sprites.len() {
                //根据z-order插入精灵到数组
                if sprite.z_order() < sprites[i].z_order() {
                    sprites.insert(i, sprite);
                    return;
                }
            }
        }
        //精灵的zOrder是最高的，放入Vec的末尾
        sprites.push(sprite);
    }

    fn draw_sprites(&mut self, g: &mut Graphics){
        //绘制所有的精灵
        for sprite in self.sprites() {
            sprite.draw(g);
        }
    }

    fn update_sprites(&mut self) {
        let sprites_num = self.sprites().len();
        //更新所有精灵
        let mut sprites_to_kill: Vec<f64> = vec![];
        for i in 0..sprites_num {
            //保存旧的精灵位置以防需要恢复
            let old_sprite_pos = *self.sprites()[i].position();
            //更新精灵
            let sprite_action = self.sprites_mut()[i].update();

            //处理SA_ADDSPRITE
            if sprite_action == SA_ADDSPRITE {
                //允许精灵添加它的精灵
                if let Some(sprite) = self.sprites()[i].add_sprite() {
                    self.add_sprite(sprite);
                }
            }

            //处理 SA_KILL
            if sprite_action == SA_KILL {
                //通知游戏精灵死亡
                self.sprite_dying(i);
                //杀死精灵
                sprites_to_kill.push(self.sprites()[i].id());
                continue;
            }

            if self.check_sprite_collision(i) {
                self.sprites_mut()[i].set_position_rect(old_sprite_pos);
            }
        }

        //删除死亡的精灵
        for sprite_id in sprites_to_kill {
            self.sprites_mut().retain(|ref s| s.id() != sprite_id);
        }
    }

    fn check_sprite_collision(&mut self, test_sprite_id: usize) -> bool {
        //检查精灵是否和其他精灵相撞
        let sprites = self.sprites_mut();
        let test_sprite = &sprites[test_sprite_id];
        for i in 0..sprites.len() {
            //不检查精灵自己
            if i == test_sprite_id {
                continue;
            }
            if test_sprite.test_collison(sprites[i].position()) {
                return self.sprite_collision(i, test_sprite_id);
            }
        }
        return false;
    }

    fn clean_up_sprites(&mut self) {
        self.sprites_mut().clear();
    }

    fn is_point_in_sprite(&mut self, x: f64, y: f64) -> Option<&Sprite> {
        for sprite in self.sprites() {
            if !sprite.hidden() && sprite.is_point_inside(x, y) {
                return Some(sprite);
            }
        }
        None
    }

    fn get_sprite(&mut self, id: f64) -> Option<&mut Sprite> {
        for sprite in self.sprites_mut() {
            if sprite.id() == id {
                return Some(sprite);
            }
        }
        None
    }

    fn initialize(&mut self) -> bool {
        true
    }

    fn end(&self) {}

    fn kill_sprite(&mut self, sprite: &Sprite) {
        if let Some(s) = self.get_sprite(sprite.id()) {
            s.kill();
        }
    }
}

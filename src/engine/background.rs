use crate::*;

pub struct ScrollingBackground {
    layers: Vec<BackgroundLayer>,
}

impl ScrollingBackground {
    pub fn new() -> ScrollingBackground {
        ScrollingBackground {
            layers: vec![],
        }
    }

    pub fn add_layer(&mut self, layer: BackgroundLayer) {
        self.layers.push(layer);
    }

    pub fn draw(&self, g: &mut Graphics) -> Result<(), String>{
        for layer in &self.layers {
            layer.draw(g);
        }
        Ok(())
    }

    pub fn update(&mut self) {
        //更新图层
        for layer in &mut self.layers {
            layer.update();
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub enum ScrollDir {
    Up,
    Right,
    Down,
    Left,
}

pub struct BackgroundLayer {
    viewport: Rect,
    speed: f64,
    direction: ScrollDir,
    bitmap: Image,
}

impl BackgroundLayer {
    pub fn new(bitmap: Image, viewport:Rect, speed: f64, direction: ScrollDir) -> BackgroundLayer {
        BackgroundLayer {
            speed,
            direction,
            bitmap,
            viewport,
        }
    }

    pub fn update(&mut self) {
        match self.direction {
            ScrollDir::Up => {
                // Move the layer up (slide the viewport down)
                self.viewport.top += self.speed;
                self.viewport.bottom += self.speed;
                if self.viewport.top > self.height() {
                    self.viewport.bottom = self.viewport.bottom - self.viewport.top;
                    self.viewport.top = 0.0;
                }
            }

            ScrollDir::Right => {
                // Move the layer right (slide the viewport left)
                self.viewport.left -= self.speed;
                self.viewport.right -= self.speed;
                if self.viewport.right < 0.0 {
                    self.viewport.left = self.width() - (self.viewport.right - self.viewport.left);
                    self.viewport.right = self.width();
                }
            }

            ScrollDir::Down => {
                // Move the layer down (slide the viewport up)
                self.viewport.top -= self.speed;
                self.viewport.bottom -= self.speed;
                if self.viewport.bottom < 0.0 {
                    self.viewport.top = self.height() - (self.viewport.bottom - self.viewport.top);
                    self.viewport.bottom = self.height();
                }
            }

            ScrollDir::Left => {
                // Move the layer left (slide the viewport right)
                self.viewport.left += self.speed;
                self.viewport.right += self.speed;
                if self.viewport.left > self.width() {
                    self.viewport.right = self.viewport.right - self.viewport.left;
                    self.viewport.left = 0.0;
                }
            }
        }
    }

    pub fn draw(&self, g: &mut Graphics){
        let (x, y) = (0.0, 0.0);
        //仅绘制通过视口看到的图层部分
        if self.viewport.top < 0.0 && self.viewport.left < 0.0 {
            //绘制分割视口，从上到下，从左到右
            //绘制左上部分(对应图片右下部分)
            g.draw_image(
                None,
                &self.bitmap,
                Some([
                    self.width() + self.viewport.left,
                    self.height() + self.viewport.top, //图像源左上角
                    -self.viewport.left,
                    -self.viewport.top,
                ]), //图像源宽高
                Some([
                    x,
                    y, //目标绘制坐标
                    -self.viewport.left,
                    -self.viewport.top,
                ]),
            );
            //绘制右上部分(对应图片左下部分)
            g.draw_image(
                None,
                &self.bitmap,
                Some([
                    0.0,
                    self.height() + self.viewport.top,
                    -self.viewport.right,
                    -self.viewport.top,
                ]),
                Some([
                    x - self.viewport.left,
                    y,
                    -self.viewport.right,
                    -self.viewport.top,
                ]),
            );
            //绘制左下部分(对应图片右上部分)
            g.draw_image(
                None,
                &self.bitmap,
                Some([
                    self.width() + self.viewport.left,
                    0.0,
                    -self.viewport.left,
                    self.viewport.bottom,
                ]),
                Some([
                    x,
                    y - self.viewport.top,
                    -self.viewport.left,
                    self.viewport.bottom,
                ]),
            );
            //绘制右下部分(对应图片左上部分)
            g.draw_image(
                None,
                &self.bitmap,
                Some([0.0, 0.0, self.viewport.right, self.viewport.bottom]),
                Some([
                    x - self.viewport.left,
                    y - self.viewport.top,
                    self.viewport.right,
                    self.viewport.bottom,
                ]),
            );
        } else if self.viewport.top < 0.0 && self.viewport.right > self.width() {
            //绘制拆开的视口，从顶部环绕到底部，从右侧环绕到左侧
            g.draw_image(
                None,
                &self.bitmap,
                Some([
                    self.viewport.left,
                    self.height() + self.viewport.top,
                    self.width() - self.viewport.left,
                    -self.viewport.top,
                ]),
                Some([x, y, self.width() - self.viewport.left, -self.viewport.top]),
            );
            g.draw_image(
                None,
                &self.bitmap,
                Some([
                    0.0,
                    self.height() + self.viewport.top,
                    self.viewport.right - self.width(),
                    -self.viewport.top,
                ]),
                Some([
                    x + (self.width() - self.viewport.left),
                    y,
                    self.viewport.right - self.width(),
                    -self.viewport.top,
                ]),
            );
            g.draw_image(
                None,
                &self.bitmap,
                Some([
                    self.viewport.left,
                    0.0,
                    self.width() - self.viewport.left,
                    self.viewport.bottom,
                ]),
                Some([
                    x,
                    y - self.viewport.top,
                    self.width() - self.viewport.left,
                    self.viewport.bottom,
                ]),
            );
            g.draw_image(
                None,
                &self.bitmap,
                Some([
                    0.0,
                    0.0,
                    self.viewport.right - self.width(),
                    self.viewport.bottom,
                ]),
                Some([
                    x + (self.width() - self.viewport.left),
                    y - self.viewport.top,
                    self.viewport.right - self.width(),
                    self.viewport.bottom,
                ]),
            );
        } else if self.viewport.bottom > self.height() && self.viewport.left < 0.0 {
            //绘制拆开的视口，从底部环绕到顶部，从左侧环绕到右侧
            g.draw_image(
                None,
                &self.bitmap,
                Some([
                    self.width() + self.viewport.left,
                    self.viewport.top,
                    -self.viewport.left,
                    self.height() - self.viewport.top,
                ]),
                Some([x, y, -self.viewport.left, self.height() - self.viewport.top]),
            );
            g.draw_image(
                None,
                &self.bitmap,
                Some([
                    0.0,
                    self.viewport.top,
                    self.viewport.right,
                    self.height() - self.viewport.top,
                ]),
                Some([
                    x - self.viewport.left,
                    y,
                    self.viewport.right,
                    self.height() - self.viewport.top,
                ]),
            );
            g.draw_image(
                None,
                &self.bitmap,
                Some([
                    self.width() + self.viewport.left,
                    0.0,
                    -self.viewport.left,
                    self.viewport.bottom - self.height(),
                ]),
                Some([
                    x,
                    y + (self.height() - self.viewport.top),
                    -self.viewport.left,
                    self.viewport.bottom - self.height(),
                ]),
            );
            g.draw_image(
                None,
                &self.bitmap,
                Some([
                    0.0,
                    0.0,
                    self.viewport.right,
                    self.viewport.bottom - self.height(),
                ]),
                Some([
                    x - self.viewport.left,
                    y + (self.height() - self.viewport.top),
                    self.viewport.right,
                    self.viewport.bottom - self.height(),
                ]),
            );
        } else if self.viewport.bottom > self.height() && self.viewport.right > self.width() {
            //绘制所有窗口，从底部环绕到顶部，从右侧环绕到左侧
            g.draw_image(
                None,
                &self.bitmap,
                Some([
                    self.viewport.left,
                    self.viewport.top,
                    self.width() - self.viewport.left,
                    self.height() - self.viewport.top,
                ]),
                Some([
                    x,
                    y,
                    self.width() - self.viewport.left,
                    self.height() - self.viewport.top,
                ]),
            );
            g.draw_image(
                None,
                &self.bitmap,
                Some([
                    0.0,
                    self.viewport.top,
                    self.viewport.right - self.width(),
                    self.height() - self.viewport.top,
                ]),
                Some([
                    x + (self.width() - self.viewport.left),
                    y,
                    self.viewport.right - self.width(),
                    self.height() - self.viewport.top,
                ]),
            );
            g.draw_image(
                None,
                &self.bitmap,
                Some([
                    self.viewport.left,
                    0.0,
                    self.width() - self.viewport.left,
                    self.viewport.bottom - self.height(),
                ]),
                Some([
                    x,
                    y + (self.height() - self.viewport.top),
                    self.width() - self.viewport.left,
                    self.viewport.bottom - self.height(),
                ]),
            );
            g.draw_image(
                None,
                &self.bitmap,
                Some([
                    0.0,
                    0.0,
                    self.viewport.right - self.width(),
                    self.viewport.bottom - self.height(),
                ]),
                Some([
                    x + (self.width() - self.viewport.left),
                    y + (self.height() - self.viewport.top),
                    self.viewport.right - self.width(),
                    self.viewport.bottom - self.height(),
                ]),
            );
        } else if self.viewport.top < 0.0 {
            //绘制拆开的视口，从顶部环绕到底部
            g.draw_image(
                None,
                &self.bitmap,
                Some([
                    self.viewport.left,
                    self.height() + self.viewport.top, //srcx, srcY
                    self.viewport.right - self.viewport.left,
                    -self.viewport.top,
                ]), //width, height
                Some([
                    x,
                    y, //destX, destY
                    self.viewport.right - self.viewport.left,
                    -self.viewport.top,
                ]),
            );
            g.draw_image(
                None,
                &self.bitmap,
                Some([
                    self.viewport.left,
                    0.0, //srcX, srcY
                    self.viewport.right - self.viewport.left,
                    self.viewport.bottom,
                ]),
                Some([
                    x,
                    y - self.viewport.top, //destX, destY
                    self.viewport.right - self.viewport.left,
                    self.viewport.bottom,
                ]),
            );
        } else if self.viewport.right > self.width() {
            //绘制拆开的视口，从右侧环绕到左侧
            let w = self.width() - self.viewport.left;
            let h = self.viewport.bottom - self.viewport.top;
            if w > 0.0 && h > 0.0 {
                g.draw_image(
                    None,
                    &self.bitmap,
                    Some([self.viewport.left, self.viewport.top, w, h]),
                    Some([x, y, w, h]),
                );
            }

            g.draw_image(
                None,
                &self.bitmap,
                Some([
                    0.0,
                    self.viewport.top,
                    self.viewport.right - self.width(),
                    self.viewport.bottom - self.viewport.top,
                ]),
                Some([
                    x + (self.width() - self.viewport.left),
                    y,
                    self.viewport.right - self.width(),
                    self.viewport.bottom - self.viewport.top,
                ]),
            );
        } else if self.viewport.bottom > self.height() {
            //绘制拆开的窗口，从底部环绕到顶部
            g.draw_image(
                None,
                &self.bitmap,
                Some([
                    self.viewport.left,
                    self.viewport.top,
                    self.viewport.right - self.viewport.left,
                    self.height() - self.viewport.top,
                ]),
                Some([
                    x,
                    y,
                    self.viewport.right - self.viewport.left,
                    self.height() - self.viewport.top,
                ]),
            );
            g.draw_image(
                None,
                &self.bitmap,
                Some([
                    self.viewport.left,
                    0.0,
                    self.viewport.right - self.viewport.left,
                    self.viewport.bottom - self.height(),
                ]),
                Some([
                    x,
                    y + (self.height() - self.viewport.top),
                    self.viewport.right - self.viewport.left,
                    self.viewport.bottom - self.height(),
                ]),
            );
        } else if self.viewport.left < 0.0 {
            //绘制拆开的视口，从左侧环绕到右侧
            g.draw_image(
                None,
                &self.bitmap,
                Some([
                    self.width() + self.viewport.left,
                    self.viewport.top,
                    -self.viewport.left,
                    self.viewport.bottom - self.viewport.top,
                ]),
                Some([
                    x,
                    y,
                    -self.viewport.left,
                    self.viewport.bottom - self.viewport.top,
                ]),
            );
            g.draw_image(
                None,
                &self.bitmap,
                Some([
                    0.0,
                    self.viewport.top,
                    self.viewport.right,
                    self.viewport.bottom - self.viewport.top,
                ]),
                Some([
                    x - self.viewport.left,
                    y,
                    self.viewport.right,
                    self.viewport.bottom - self.viewport.top,
                ]),
            );
        } else {
            //一次性绘制整个视口
            g.draw_image(
                None,
                &self.bitmap,
                Some([
                    self.viewport.left,
                    self.viewport.top,
                    self.viewport.right - self.viewport.left,
                    self.viewport.bottom - self.viewport.top,
                ]),
                Some([
                    x,
                    y,
                    self.viewport.right - self.viewport.left,
                    self.viewport.bottom - self.viewport.top,
                ]),
            );
        }
    }

    pub fn set_speed(&mut self, speed: f64) {
        self.speed = speed;
    }

    pub fn set_direction(&mut self, direction: ScrollDir) {
        self.direction = direction;
    }

    pub fn set_viewport(&mut self, viewport: Rect) {
        self.viewport = viewport;
    }

    pub fn width(&self) -> f64 {
        self.bitmap.width()
    }

    pub fn height(&self) -> f64 {
        self.bitmap.height()
    }
}

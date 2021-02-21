
use crate::drawing::Rectangle;
use crate::drawing::Framebuffer;
use crate::drawing::Pixel;
use crate::drawing::Vector;

pub struct Pong {
    player: (Rectangle, Pixel),
    npc: (Rectangle, Pixel),
    ball: (Rectangle, Vector, Pixel),
    bgcolor: (Rectangle, Pixel),
    paused: bool
}

impl Pong {
    pub fn new(playc: Pixel, npcc: Pixel, ballc: Pixel, bgcolor: Pixel) -> Self {
        Self {
            player: (Rectangle::new(15, 150, 25, 100), playc),
            npc: (Rectangle::new(600, 150, 25, 100), npcc),
            ball: (Rectangle::new(320, 240, 25, 25), Vector::new(15, -2), ballc),
            bgcolor: (Rectangle::new(0, 0, 640, 480), bgcolor),
            paused: true
        }
    }
    pub fn advance(&mut self) {
        if !self.paused {
            let nx = self.ball.0.x as isize + self.ball.1.x;
            let ny = self.ball.0.y as isize + self.ball.1.y;

            if nx >= 580 || nx <= 35 {
                self.ball.1.x = -self.ball.1.x;
            }
            if ny >= 400 || ny <= 50 {
                self.ball.1.y = -self.ball.1.y;
            }
            self.ball.0.x = nx as usize;
            self.ball.0.y = ny as usize;
        }
    }
    pub fn draw(&self, fb: &mut Framebuffer) {
        fb.fill_rect(&self.bgcolor.0, &self.bgcolor.1);
        fb.fill_rect(&self.player.0, &self.player.1);
        fb.fill_rect(&self.npc.0, &self.npc.1);
        fb.fill_rect(&self.ball.0, &self.ball.2);
    }
    pub fn move_player_up(&mut self, y: usize) {
        if !self.paused {
            self.player.0.y -= y;
        }
    }
    pub fn move_player_down(&mut self, y: usize) {
        if !self.paused {
            self.player.0.y += y;
        }
    }
    pub fn move_ball_left(&mut self, x: usize) {
        if !self.paused {
            self.ball.0.x -= x;
        }
    }
    pub fn move_ball_right(&mut self, x: usize) {
        if !self.paused {
            self.ball.0.x += x;
        }
    }
    pub fn move_ball_up(&mut self, y: usize) {
        if !self.paused {
            self.ball.0.y -= y;
        }
    }
    pub fn move_ball_down(&mut self, y: usize) {
        if !self.paused {
            self.ball.0.y += y;
        }
    }
    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }
    pub fn is_paused(&self) -> bool {
        self.paused
    }
}

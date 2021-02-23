use crate::drawing::{Framebuffer, Pixel, Rectangle, Vector};

struct Obj {
    pub location: Rectangle,
    pub color: Pixel
}
impl Obj {
    pub fn new(location: Rectangle, color: Pixel) -> Self {
        Self {
            location, color
        }
    }
}

pub struct Pong {
	player: Obj,
	npc: Obj,
	ball: Obj,
    ball_direction: Vector,
	bgcolor: Pixel,
	paused: bool
}

impl Pong {
	pub fn new(playc: Pixel, npcc: Pixel, ballc: Pixel, bgcolor: Pixel) -> Self {
		Self {
			player: Obj::new(Rectangle::new(15, 150, 25, 100), playc),
			npc: Obj::new(Rectangle::new(600, 150, 25, 100), npcc),
			ball: Obj::new(Rectangle::new(320, 240, 25, 25), ballc),
            ball_direction: Vector::new(15, -2),
			bgcolor,
			paused: true,
		}
	}
	pub fn reset(&mut self) {
		self.ball.location = Rectangle::new(320, 240, 25, 25);
        self.ball_direction = Vector::new(15, -2);
        self.paused = true;
	}
	pub fn advance_frame(&mut self) {
		if !self.paused {
            self.move_ball(self.ball_direction.x, self.ball_direction.y);
			let miss = 
			if self.ball.location.x < 40 {
				// This means we're in the player's paddle location. Let's
				// see if this is a hit or a miss!
				let paddle = (self.player.location.y, self.player.location.y + self.player.location.height);
				let ball = (self.ball.location.y, self.ball.location.y + self.ball.location.height);

				if paddle.0 <= ball.0 && paddle.1 >= ball.0 {
					false
				}
				else if paddle.0 <= ball.1 && paddle.1 >= ball.1 {
					false
				}
				else {
					true
				}
			}
			else {
				false
			};
			if miss {
				self.reset();
				self.paused = true;
			}
			else {
				if self.ball.location.x < 40 || self.ball.location.x > 580 {
					self.ball_direction.x = -self.ball_direction.x;
				}
				if self.ball.location.y < 20 || self.ball.location.y > 430 {
					self.ball_direction.y = -self.ball_direction.y;
				}
				let new_loc = self.ball.location.y - self.npc.location.height / 2;
				self.npc.location.y = if new_loc > 0 { new_loc } else { 0 };
			}
		}
	}
	pub fn draw(&self, fb: &mut Framebuffer) {
		fb.fill_rect(&Rectangle::new(0, 0, 640, 480), &self.bgcolor);
		fb.fill_rect(&self.player.location, &self.player.color);
		fb.fill_rect(&self.npc.location, &self.npc.color);
		fb.fill_rect(&self.ball.location, &self.ball.color);
	}
    pub fn move_player(&mut self, y: i32) {
        if !self.paused {
            let new_loc = self.player.location.y + y;
            self.player.location.y = if new_loc < 0 { 0 } else if new_loc > 400 { 400 } else { new_loc };
        }
    }
    pub fn move_ball(&mut self, x: i32, y: i32) {
        if !self.paused {
            self.ball.location.x += x;
            self.ball.location.y += y;
        }
    }
	pub fn toggle_pause(&mut self) {
		self.paused = !self.paused;
	}
	pub fn is_paused(&self) -> bool {
		self.paused
	}
}


// use crate::syscall::syscall;

#[repr(C)]
#[derive(Clone,Copy)]
pub struct Pixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

pub type Color = Pixel;

impl Pixel {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self {
            r, g, b, a: 255
        }
    }
}

pub struct Vector {
    pub x: i32,
    pub y: i32
}

impl Vector {
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            x,  y
        }
    }
}

pub struct Rectangle {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rectangle {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x, y, width, height
        }
    }
}
pub struct Framebuffer {
    pixels: *mut Pixel
}

impl Framebuffer {
    pub fn new(pixels: *mut Pixel) -> Self {
        Self { pixels }
    }
    pub fn set(&mut self, x: i32, y: i32, pixel: &Pixel) {
        unsafe {
            if x < 640 && y < 480 {
                let v = (y * 640 + x) as isize;
                self.pixels.offset(v).write(*pixel);
            }
        }
    }
    pub fn fill_rect(&mut self, rect: &Rectangle, color: &Pixel) {
        let row_start = rect.y;
        let row_finish = row_start + rect.height;
        let col_start = rect.x;
        let col_finish = col_start + rect.width;
        for row in row_start..row_finish {
            for col in col_start..col_finish {
                self.set(col, row, color);
            }
        }
    }
}

pub fn lerp(value: i32, mx1: i32, mx2: i32) -> i32 {
    let r = (value as f64) / (mx1 as f64);
	return r as i32 * mx2;
}


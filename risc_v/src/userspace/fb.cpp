#include <printf.h>
#include <syscall.h>
#include <input-event-codes.h>


#define MAX_EVENTS 100000
#define cos(x)	  table_cos(x)
// #define cos(x)	  taylor_cos(x)
#define min(x, y) ((x < y) ? x : y)
#define max(x, y) ((x > y) ? x : y)

using u8 = unsigned char;
using i8 = signed char;
using u16 = unsigned short;
using i16 = signed short;
using u32 = unsigned int;
using i32 = signed int;
using u64 = unsigned long;
using i64 = signed long;
using f64 = double;
using f32 = float;

struct Pixel {
	u8 r;
	u8 g;
	u8 b;
	u8 a;
};

struct Event {
	u16 event_type;
	u16 code;
	u32 value;
} events[MAX_EVENTS];

void fill_rect(Pixel *fb, u32 x, u32 y, u32 width, u32 height, Pixel &color);
void stroke_rect(Pixel *fb, u32 x, u32 y, u32 width, u32 height, Pixel &color, u32 size);
void set_pixel(Pixel *fb, u32 x, u32 y, Pixel &color);
void draw_cosine(Pixel *fb, u32 x, u32 y, u32 width, u32 height, Pixel &color);
void draw_circle(Pixel *fb, u32 x, u32 y, f64 r, Pixel &color);

const u64 noevt_slptm = 10000;
const u64 evt_slptm   = 10000;

struct Rect {
	u32 x;
	u32 y;
	u32 width;
	u32 height;
};

u32 lerp(u32 val, u32 mx1, u32 mx2) {
	f64 r = val / static_cast<f64>(mx1);
	return r * mx2;
}

int main()
{
	bool mouse_down = true;
	int VERSION = -1;
	printf("(%d): TESTING FRAMEBUFFER FROM USERSPACE\n", VERSION);
	Pixel *fb = (Pixel *)syscall_get_fb(6);
	Pixel blue_color = {0, 0, 255, 255};
	Pixel red_color = {255, 0, 0, 255};
	Pixel green_color = {0, 255, 0, 255};
	Pixel white_color = {255, 255, 255, 255};
	Pixel orange_color = {255, 150, 0, 255};
	u32 x = 0;
	u32 y = 0;
	u32 width = 40;
	u32 height = 50;
	unsigned long i = 0;

	fill_rect(fb, 0, 0, 640, 480, white_color);
	stroke_rect(fb, 10, 10, 20, 20, blue_color, 5);
	stroke_rect(fb, 50, 50, 40, 40, green_color, 10);
	stroke_rect(fb, 150, 150, 140, 140, red_color, 15);
	draw_cosine(fb, 0, 400, 500, 50, red_color);
	syscall_inv_rect(6, x, y, 640, 480);
	i64 abs;
	do {
		
		if ((abs = syscall_get_abs(events, MAX_EVENTS)) < 1) {
			syscall_sleep(noevt_slptm);
		}
		for (i64 z = 0;z < abs;z++) {
			Event &ev = events[z];
			if (ev.code == ABS_X) {
				x = lerp(ev.value & 0xffff, 32767, 640);
				fill_rect(fb, x, y, 5, 5, orange_color);
			}
			else if (ev.code == ABS_Y) {
				y = lerp(ev.value & 0xffff, 32767, 480);
				fill_rect(fb, x, y, 5, 5, orange_color);
			}
		}
		syscall_inv_rect(6, 0, 0, 640, 480);
		// if (i++ >= 100) {
		// 	i = 0;
		// 	syscall_inv_rect(6, 0, 0, 640, 480);
		// }
	} while (true);
	return 0;
}

void set_pixel(Pixel *fb, u32 x, u32 y, Pixel &color) {
	if (x < 640 && y < 480) {
		fb[y * 640 + x] = color;
	}
}

void fill_rect(Pixel *fb, u32 x, u32 y, u32 width, u32 height, Pixel &color) {
	for (u32 row = y; row < (y+height);row++) {
		for (u32 col = x; col < (x+width);col++) {
			set_pixel(fb, col, row, color);
		}
	}
}

void stroke_rect(Pixel *fb, u32 x, u32 y, u32 width, u32 height, Pixel &color, u32 size) {
   // Essentially fill the four sides.
   // Top
   fill_rect(fb, x, y, width, size, color);
   // Bottom
   fill_rect(fb, x, y + height, width, size, color);
   // Left
   fill_rect(fb, x, y, size, height, color);
   // Right
   fill_rect(fb, x + width, y, size, height + size, color);
}

f64 table_cos(f64 angle_degrees) {
	const f64 COS_TABLE[] = {
		1.0,
		0.9962,
		0.9848,
		0.9659,
		0.9397,
		0.9063,
		0.8660,
		0.8191,
		0.7660,
		0.7071,
		0.6428,
		0.5736,
		0.5000,
		0.4226,
		0.3420,
		0.2558,
		0.1736,
		0.0872,
		0.0,
		-0.0872,
		-0.1736,
		-0.2558,
		-0.3420,
		-0.4226,
		-0.5000,
		-0.5736,
		-0.6428,
		-0.7071,
		-0.7660,
		-0.8191,
		-0.8660,
		-0.9063,
		-0.9397,
		-0.9659,
		-0.9848,
		-0.9962,
		-1.0,
		-0.9962,
		-0.9848,
		-0.9659,
		-0.9397,
		-0.9063,
		-0.8660,
		-0.8191,
		-0.7660,
		-0.7071,
		-0.6428,
		-0.5736,
		-0.5000,
		-0.4226,
		-0.3420,
		-0.2558,
		-0.1736,
		-0.0872,
		0.0,
		0.0872,
		0.1736,
		0.2558,
		0.3420,
		0.4226,
		0.5000,
		0.5736,
		0.6428,
		0.7071,
		0.7660,
		0.8191,
		0.8660,
		0.9063,
		0.9397,
		0.9659,
		0.9848,
		0.9962,
		1.0,
	};
	u32 lookup_ang = angle_degrees / 5;
	return COS_TABLE[lookup_ang];
}

f64 taylor_cos(f64 angle_degrees) {
	f64 x = 3.14159265359 * angle_degrees / 180.0;
	f64 result = 1.0;
	f64 inter = 1.0;
	f64 num = x * x;
	for (int i = 1;i <= 6;i++) {
		u64 comp = 2 * i;
		u64 den = comp * (comp - 1);
		inter *= num / den;
		if ((i & 1) == 0) {
			result += inter;
		}
		else {
			result -= inter;
		}
	}
	return result;
}

f64 sin(f64 angle_degrees) {
	return cos(90.0 - angle_degrees);
}

void draw_cosine(Pixel *fb, u32 x, u32 y, u32 width, u32 height, Pixel &color) {
	for (u32 i = 1; i <= width;i++) {
		f64 fy = -cos(i % 360);
		f64 yy = fy / 2.0 * height;
		u32 nx = x + i;
		u32 ny = yy + y;
		// printf("Cos %u = %lf, x: %u, y: %u\n", (i % 360), fy, nx, ny);
		fill_rect(fb, nx, ny, 2, 2, color);
	}
}




void draw_circle(Pixel *fb, u32 x, u32 y, f64 r, Pixel &color)
{

}


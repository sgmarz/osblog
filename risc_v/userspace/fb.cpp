#include <sys/stat.h>
#include <fcntl.h>
#include <unistd.h>
#include <cmath>
#include <cstdio>
#include <input-event-codes.h>


#define MAX_EVENTS 100
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
};

void fill_rect(Pixel *fb, u32 x, u32 y, u32 width, u32 height, Pixel &color);
void stroke_rect(Pixel *fb, u32 x, u32 y, u32 width, u32 height, Pixel &color, u32 size);
void set_pixel(Pixel *fb, u32 x, u32 y, Pixel &color);
void draw_cosine(Pixel *fb, u32 x, u32 y, u32 width, u32 height, Pixel &color);
void draw_circle(Pixel *fb, u32 x, u32 y, f64 r, Pixel &color);

const u64 noevt_slptm = 10000;
const u64 evt_slptm   = 10000;

#define FB_DEV "/dev/fb"
#define BUT_DEV "/dev/butev"
#define ABS_DEV "/dev/absev"

struct Rect {
	u32 x;
	u32 y;
	u32 width;
	u32 height;
};

constexpr u32 lerp(u32 val, u32 mx1, u32 mx2) {
	f64 r = val / static_cast<f64>(mx1);
	return r * mx2;
}

int main()
{
	Event *events = new Event[100];
	bool pressed = false;
	int fb = open(FB_DEV, O_RDWR);
	int but = open(BUT_DEV, O_RDONLY);
	int abs = open(ABS_DEV, O_RDONLY);
	if (fb < 0) {
		printf("Unable to open framebuffer %s.\n", FB_DEV);
		return -1;
	}
	if (but < 0) {
		printf("Unable to open button events %s.\n", BUT_DEV);
		return -1;
	}
	if (abs < 0) {
		printf("Unable to open absolute events %s.\n", ABS_DEV);
		return -1;

	}
	close(fb);
	close(but);
	close(abs);
	delete [] events;
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


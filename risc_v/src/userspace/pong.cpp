#include <printf.h>
#include <syscall.h>
#include <input-event-codes.h>



int main()
{
	bool pressed = false;
	Pixel *fb = (Pixel *)syscall_get_fb(6);
	Pixel white_color = {255, 255, 255, 255};
	Pixel black_color = {0, 0, 0, 255};
	Pixel current_color = {255, 150, 0, 255};
	Pixel ball_color = white_color;
	Pixel player_color = {255, 0, 0, 255};
	Pixel computer_color = {0, 0, 255, 255};
	Rect player = {10, 200, 35, 150};
	Rect computer = {550, 200, 35, 150};
	Rect ball = {510, 50, 25, 25};
	Vec2 ball_direction = {-15, 3};
	Pixel blue_color = {0, 0, 255, 255};
	u32 x = 0;
	u32 y = 0;
	u32 num_events;

	fill_rect(fb, 0, 0, 640, 480, black_color);
	syscall_inv_rect(6, 0, 0, 640, 480);
	do
	{
		if ((num_events = syscall_get_key(events, MAX_EVENTS)) > 0)
		{
			for (u32 z = 0; z < num_events; z++)
			{
				Event &ev = events[z];
				switch (ev.code)
				{
				case BTN_MOUSE:
					pressed = (ev.value & 1) == 1;
					break;
				case KEY_O:
					ball.x = 510;
					ball.y = 50;
					ball_direction.x = -15;
					ball_direction.y = 3;
					break;
				case KEY_B:
					ball_color = Pixel{0, 0, 255, 255};
					break;
				case KEY_G:
					ball_color = Pixel{0, 255, 0, 255};
					break;
				case KEY_R:
					ball_color = Pixel{255, 0, 0, 255};
					break;
				case KEY_W:
					if (ev.value == 1)
					{
						fill_rect(fb, player, black_color);
						player.translate(0, -20);
					}
					break;
				case KEY_S:
					if (ev.value == 1)
					{
						fill_rect(fb, player, black_color);
						player.translate(0, 20);
					}
					break;
				}
			}
		}
		fill_rect(fb, ball, black_color);
		// Move the ball
		ball.x += ball_direction.x;
		ball.y += ball_direction.y;

		if (ball.x >= computer.x)
		{
			ball_direction.x = -15;
		}
		else if (ball.x <= player.x)
		{
			ball_direction.x = 15;
		}
		fill_rect(fb, ball, ball_color);
		fill_rect(fb, player, player_color);
		fill_rect(fb, computer, computer_color);
		syscall_inv_rect(6, 0, 0, 640, 480);

		syscall_sleep(noevt_slptm);
		// if ((num_events = syscall_get_abs(events, MAX_EVENTS)) < 1)
		// {
		// 	syscall_sleep(noevt_slptm);
		// 	continue;
		// }
		// for (u32 z = 0; z < num_events; z++)
		// {
		// 	Event &ev = events[z];
		// 	if (ev.code == ABS_X)
		// 	{
		// 		x = lerp(ev.value & 0x7fff, 32767, 640);
		// 	}
		// 	else if (ev.code == ABS_Y)
		// 	{
		// 		y = lerp(ev.value & 0x7fff, 32767, 480);
		// 	}
		// 	if (pressed)
		// 	{
		// 		fill_rect(fb, x, y, 5, 5, current_color);
		// 	}
		// }
		// if (pressed)
		// {
		// 	syscall_inv_rect(6, 0, 0, 640, 480);
		// }
	} while (true);
	return 0;
}

void set_pixel(Pixel *fb, int x, int y, Pixel &color)
{
	if (x < 640 && y < 480)
	{
		fb[y * 640 + x] = color;
	}
}

void fill_rect(Pixel *fb, Rect &rect, Pixel &color)
{
	fill_rect(fb, rect.x, rect.y, rect.width, rect.height, color);
}

void fill_rect(Pixel *fb, u32 x, u32 y, u32 width, u32 height, Pixel &color)
{
	for (u32 row = y; row < (y + height); row++)
	{
		for (u32 col = x; col < (x + width); col++)
		{
			set_pixel(fb, col, row, color);
		}
	}
}

void stroke_rect(Pixel *fb, u32 x, u32 y, u32 width, u32 height, Pixel &color, u32 size)
{
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


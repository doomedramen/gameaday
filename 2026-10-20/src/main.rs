use minifb::{Key, Window, WindowOptions};
use rand::Rng;
use std::time::Instant;

const WIDTH: usize = 640;
const HEIGHT: usize = 480;

#[derive(Clone, Debug)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Rect {
    fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.w &&
        self.x + self.w > other.x &&
        self.y < other.y + other.h &&
        self.y + self.h > other.y
    }
}

fn main() {
    let mut window = Window::new(
        "Brick Bounce - GameADay Day 1",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
    .expect("Failed to create window");
    
    window.set_target_fps(60);
    
    // Paddle
    let mut paddle = Rect { x: (WIDTH as f32)/2.0 - 40.0, y: (HEIGHT as f32) - 30.0, w: 80.0, h: 10.0 };
    let paddle_speed: f32 = 400.0;
    
    // Ball
    let mut ball = Rect { x: (WIDTH as f32)/2.0 - 4.0, y: (HEIGHT as f32)/2.0, w: 8.0, h: 8.0 };
    let mut ball_vel_x: f32 = 200.0;
    let mut ball_vel_y: f32 = -200.0;
    
    // Bricks
    let mut bricks: Vec<Rect> = Vec::new();
    let brick_w = 60.0;
    let brick_h = 20.0;
    let rows = 5;
    let cols = 8;
    for r in 0..rows {
        for c in 0..cols {
            bricks.push(Rect {
                x: 40.0 + (c as f32) * (brick_w + 4.0),
                y: 40.0 + (r as f32) * (brick_h + 4.0),
                w: brick_w,
                h: brick_h,
            });
        }
    }
    
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
    let mut last = Instant::now();
    let mut lives = 3;
    let mut game_over = false;
    let mut game_won = false;
    
    while window.is_open() && !window.is_key_down(Key::Escape) {
        if game_over || game_won {
            // Simple end screen
            for pixel in buffer.iter_mut().take(WIDTH * HEIGHT) {
                *pixel = 0x000000;
            }
            // Draw simple pattern
            let color = if game_won { 0x00FF00 } else { 0xFF0000 };
            for i in 0..(WIDTH * HEIGHT).min(1000) {
                buffer[i] = color;
            }
            window.update_with_buffer(&buffer, WIDTH, HEIGHT).expect("Failed to update");
            continue;
        }
        
        let now = Instant::now();
        let dt = (now - last).as_secs_f32();
        last = now;
        
        // Paddle movement
        if window.is_key_down(Key::Left) && paddle.x > 0.0 {
            paddle.x -= paddle_speed * dt;
        }
        if window.is_key_down(Key::Right) && paddle.x + paddle.w < WIDTH as f32 {
            paddle.x += paddle_speed * dt;
        }
        
        // Ball movement
        ball.x += ball_vel_x * dt;
        ball.y += ball_vel_y * dt;
        
        // Wall collisions
        if ball.x <= 0.0 || ball.x + ball.w >= WIDTH as f32 {
            ball_vel_x = -ball_vel_x;
        }
        if ball.y <= 0.0 {
            ball_vel_y = -ball_vel_y;
        }
        if ball.y + ball.h >= HEIGHT as f32 {
            lives -= 1;
            if lives <= 0 {
                game_over = true;
            } else {
                ball.x = (WIDTH as f32)/2.0 - 4.0;
                ball.y = (HEIGHT as f32)/2.0;
                ball_vel_x = 200.0 * if rand::thread_rng().gen_bool(0.5) { 1.0 } else { -1.0 };
                ball_vel_y = -200.0;
            }
        }
        
        // Paddle collision
        if ball.intersects(&paddle) && ball_vel_y > 0.0 {
            ball_vel_y = -ball_vel_y;
            // Add some angle based on hit position
            let hit_pos = (ball.x + ball.w/2.0 - paddle.x) / paddle.w - 0.5;
            ball_vel_x += hit_pos * 100.0;
        }
        
        // Brick collisions
        let mut hit_brick = None;
        for (i, brick) in bricks.iter().enumerate() {
            if ball.intersects(brick) {
                hit_brick = Some(i);
                break;
            }
        }
        if let Some(i) = hit_brick {
            bricks.remove(i);
            ball_vel_y = -ball_vel_y;
            if bricks.is_empty() {
                game_won = true;
            }
        }
        
        // Clear buffer
        for pixel in buffer.iter_mut().take(WIDTH * HEIGHT) {
            *pixel = 0x001122;
        }
        
        // Draw paddle
        for py in 0..(paddle.h as usize) {
            for px in 0..(paddle.w as usize) {
                let x = (paddle.x as usize) + px;
                let y = (paddle.y as usize) + py;
                if x < WIDTH && y < HEIGHT {
                    buffer[y * WIDTH + x] = 0xFFFFFF;
                }
            }
        }
        
        // Draw ball
        for py in 0..(ball.h as usize) {
            for px in 0..(ball.w as usize) {
                let x = (ball.x as usize) + px;
                let y = (ball.y as usize) + py;
                if x < WIDTH && y < HEIGHT {
                    buffer[y * WIDTH + x] = 0xFFFF00;
                }
            }
        }
        
        // Draw bricks
        for brick in bricks.iter() {
            for py in 0..(brick.h as usize) {
                for px in 0..(brick.w as usize) {
                    let x = (brick.x as usize) + px;
                    let y = (brick.y as usize) + py;
                    if x < WIDTH && y < HEIGHT {
                        buffer[y * WIDTH + x] = 0xFF4444;
                    }
                }
            }
        }
        
        window.update_with_buffer(&buffer, WIDTH, HEIGHT).expect("Failed to update");
    }
}

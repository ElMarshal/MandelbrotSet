use std::path::Path;
use std::fs::File;
use std::io::{BufWriter, stdout, Write};
use std::time;
use std::sync::{Mutex, Arc};
use rand::Rng;
use std::thread;

type Real = f32;

#[derive(Copy, Clone)]
struct Complex {
    r: Real,
    i: Real,
}

impl Complex {
    fn squared(&self) -> Complex {
        Complex {r: self.r*self.r - self.i*self.i, i: 2.0*self.r*self.i}
    }

    fn add(&self, rhs: &Complex) -> Complex {
        Complex {r: self.r + rhs.r, i: self.i + rhs.i}
    }

    fn length(&self) -> Real {
        (self.r*self.r + self.i*self.i).sqrt()
    }
}

#[derive(Copy, Clone)]
struct Color {
    r: Real,
    g: Real,
    b: Real,
    a: Real,
}

impl Color {
    fn new() -> Color {
        Color {r:0.0, g:0.0, b:0.0, a:0.0}
    }

    fn add(&mut self, rhs: Color) {
        self.r += rhs.r;
        self.g += rhs.g;
        self.b += rhs.b;
        self.a += rhs.a;
    }

    fn divide(&mut self, value: Real) {
        self.r /= value;
        self.g /= value;
        self.b /= value;
        self.a /= value;
    }
}

fn save_image(color_buffer: &[Color], width: usize, height: usize, path: &str) {
    let path = Path::new(path);
    let file = File::create(path).unwrap();
    let ref mut w = BufWriter::new(file);
    
    let mut encoder = png::Encoder::new(w, width as u32, height as u32);
    encoder.set_color(png::ColorType::RGBA);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    
    let mut rgba_data = vec![0u8; 4 * width * height];
    for y in 0..height {
        for x in 0..width {
            rgba_data[4 * (y * width + x) + 0] = (color_buffer[y * width + x].r * 255.0) as u8;
            rgba_data[4 * (y * width + x) + 1] = (color_buffer[y * width + x].g * 255.0) as u8;
            rgba_data[4 * (y * width + x) + 2] = (color_buffer[y * width + x].b * 255.0) as u8;
            rgba_data[4 * (y * width + x) + 3] = (color_buffer[y * width + x].a * 255.0) as u8;
        }
    }

    writer.write_image_data(&rgba_data).unwrap();
}

// Prints the progress [0:100] as a bar in the console
fn print_progress(progress: u32) {
    let mut progress_bar = String::from("[");
    for i in 0..50 {
        if i <= progress/2 {
            progress_bar.push('=');
        }
        else {
            progress_bar.push(' ');
        }
    }
    progress_bar.push_str("]");
    print!("\r{} {}%  ", progress_bar, progress);
    stdout().flush().unwrap();
}

fn clamp<T>(value: T, min: T, max: T) -> T 
where T: PartialOrd {
    if value < min {
        return min;
    }
    else if value > max {
        return max;
    }
    value
}

fn divide_roundup(numinator: usize, denominator: usize) -> usize {
    if(numinator%denominator == 0) {
        return numinator/denominator;
    }
    numinator/denominator+1
}

// Render Parameters
// COLOR PALLETE SOURCE: https://stackoverflow.com/a/16505538/9218594
const COLOR_PALETTE: [Color; 16] = [
    Color {r:0.26, g:0.1,  b:0.06, a:1.0}, 
    Color {r:0.1,  g:0.03, b:0.1,  a:1.0}, 
    Color {r:0.3,  g:0.01, b:0.18, a:1.0}, 
    Color {r:0.02, g:0.02, b:0.28, a:1.0}, 
    Color {r:0.0,  g:0.03, b:0.4,  a:1.0}, 
    Color {r:0.05, g:0.17, b:0.54, a:1.0}, 
    Color {r:0.1,  g:0.3,  b:0.7,  a:1.0}, 
    Color {r:0.25, g:0.5,  b:0.82, a:1.0}, 
    Color {r:0.53, g:0.71, b:0.9,  a:1.0}, 
    Color {r:0.83, g:0.93, b:0.97, a:1.0}, 
    Color {r:0.95, g:0.91, b:0.75, a:1.0}, 
    Color {r:0.97, g:0.78, b:0.37, a:1.0}, 
    Color {r:1.0,  g:0.67, b:0.0,  a:1.0}, 
    Color {r:0.8,  g:0.5,  b:0.0,  a:1.0}, 
    Color {r:0.6,  g:0.34, b:0.0,  a:1.0}, 
    Color {r:0.42, g:0.2,  b:0.02, a:1.0} 
    ];
const MAX_ITERATIONS: u32 = 250;
const MAX_LENGTH: Real = 2.0;

#[derive(Copy, Clone)]
struct ThreadDescryptor {
    off_x : usize,
    off_y : usize, 
    width : usize, // Thread width
    height : usize, // Thread height
    color_buffer_width : usize,
    color_buffer_height : usize,
    sample_count : usize,
    center_x : Real,
    center_y : Real,
    view_width : Real,
    view_height : Real,
}

impl ThreadDescryptor {
    fn new() -> ThreadDescryptor {
        ThreadDescryptor {
            off_x: 0, off_y: 0,
            width: 0, height: 0,
            color_buffer_width: 0, color_buffer_height: 0,
            sample_count: 0,
            center_x: 0.0, center_y: 0.0,
            view_width: 0.0, view_height: 0.0
        }
    }
}

fn thread_worker(color_buffer : Arc<Mutex<Vec<Color>>>, descrypt : ThreadDescryptor) {
    let mut temp_color_buffer = vec![Color::new(); descrypt.width * descrypt.height];
    let mut rng = rand::thread_rng();

    // Render the fractal into the temporary color buffer
    for y in 0..descrypt.height {
        for x in 0..descrypt.width {
            let mut pixel_color = Color::new();
            for _ in 0..descrypt.sample_count {
                let norm_pos_x = (((x+descrypt.off_x) as Real) + rng.gen_range(-0.5, 0.5))/(descrypt.color_buffer_width as Real) * 2.0 - 1.0; // [-1:1]
                let norm_pos_y = (((y+descrypt.off_y) as Real) + rng.gen_range(-0.5, 0.5))/(descrypt.color_buffer_height as Real) * 2.0 - 1.0; // [-1:1]
                let mut pos = Complex {r: 0.0, i: 0.0};
                pos.r = descrypt.center_x + norm_pos_x * descrypt.view_width / 2.0; // real axis
                pos.i = descrypt.center_y + norm_pos_y * descrypt.view_height / 2.0; // imaginary axis
                let mut iterations: u32 = 0;
                let mut temp = Complex {r: 0.0, i: 0.0};
                while temp.length() <= MAX_LENGTH && iterations < MAX_ITERATIONS {
                    temp = temp.squared().add(&pos);
                    iterations += 1;
                }
                pixel_color.add(COLOR_PALETTE[(iterations%16) as usize]);
            }
            pixel_color.divide(descrypt.sample_count as Real);
            temp_color_buffer[y * descrypt.width + x] = pixel_color;
        }
    }

    // copy the temporary color buffer after locking the color_buffer mutex
    let mut cb = color_buffer.lock().unwrap();
    for y in 0..descrypt.height {
        for x in 0..descrypt.width {
            cb[(y+descrypt.off_y) * descrypt.color_buffer_width + (x+descrypt.off_x)] = temp_color_buffer[y * descrypt.width + x];
        }
    }

}

fn main() {
    const BUFFER_WIDTH: usize = 1366;
    const BUFFER_HEIGHT: usize = 768;
    const BUFFER_ASPECT_RATIO: Real = (BUFFER_WIDTH as Real) / (BUFFER_HEIGHT as Real);
    const SAMPLE_COUNT: usize = 16;
    const CENTER_X: Real = -0.7453;
    const CENTER_Y: Real = 0.1127;
    const VIEW_WIDTH: Real = 6.5E-4 * BUFFER_ASPECT_RATIO;
    const VIEW_HEIGHT: Real = 6.5E-4;
    const THREAD_WIDTH: usize = 128;
    const THREAD_HEIGHT: usize = 128;
    const THREAD_COUNT: usize = 4;

    // Row major
    let color_buffer = Arc::new(Mutex::new(vec![Color::new(); BUFFER_WIDTH * BUFFER_HEIGHT]));

    println!("Drawing the buffer...");
    let start_time = time::Instant::now();

    // Fill threads descryptors
    let mut threads_descryptors = Vec::new();
    for y in 0..divide_roundup(BUFFER_HEIGHT, THREAD_HEIGHT) {
        for x in 0..divide_roundup(BUFFER_WIDTH, THREAD_WIDTH) {
            let mut new_descryptor = ThreadDescryptor::new();
            new_descryptor.off_x = x * THREAD_WIDTH;
            new_descryptor.off_y = y * THREAD_HEIGHT;
            let max_width = BUFFER_WIDTH - x*THREAD_WIDTH;
            let max_height = BUFFER_HEIGHT - y*THREAD_HEIGHT;
            new_descryptor.height = clamp(THREAD_HEIGHT, 0, max_height);
            new_descryptor.width = clamp(THREAD_WIDTH, 0, max_width);
            new_descryptor.color_buffer_width = BUFFER_WIDTH;
            new_descryptor.color_buffer_height = BUFFER_HEIGHT;
            new_descryptor.sample_count = SAMPLE_COUNT;
            new_descryptor.center_x = CENTER_X;
            new_descryptor.center_y = CENTER_Y;
            new_descryptor.view_width = VIEW_WIDTH;
            new_descryptor.view_height = VIEW_HEIGHT;
            threads_descryptors.push(new_descryptor);
        }
    }

    // Spawn threads_descryptors.len() threads, THREAD_COUNT at a time
    for i in 0..divide_roundup(threads_descryptors.len(), THREAD_COUNT) {
        let max_thread_count = clamp(THREAD_COUNT, 0, threads_descryptors.len() - i*THREAD_COUNT);
        let mut threads = Vec::new();

        for t in 0..max_thread_count {
            let descryptor = threads_descryptors[i*THREAD_COUNT + t];
            let color_buffer_clone = color_buffer.clone();
            threads.push(thread::spawn(move || thread_worker(color_buffer_clone, descryptor.clone())));
        }

        // join all threads
        for thread in threads {
            thread.join().unwrap();
        }

        let progress = (i*THREAD_COUNT)*100/threads_descryptors.len();
        print_progress(progress as u32);
    }

    print_progress(100);
    let duration = time::Instant::now().duration_since(start_time).as_secs();
    println!("\nFinished rendering in {}h{}m{}s", (duration/60/60), (duration/60)%60, duration%(60*60));

    let cb = color_buffer.lock().unwrap();
    save_image(&cb, BUFFER_WIDTH, BUFFER_HEIGHT, "output/image.png");
    println!("Saved buffer to image.png");
}

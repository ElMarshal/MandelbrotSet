use std::path::Path;
use std::fs::File;
use std::io::{BufWriter, stdout, Write};
use std::time;
use std::sync::{Mutex, Arc, mpsc::channel, mpsc::Sender};
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
struct Vec2<T> {
    x: T,
    y: T,
}

impl Vec2<usize> {
    fn new() -> Vec2<usize> {
        Vec2{x:0, y:0}
    }
}

impl Vec2<Real> {
    fn new() -> Vec2<Real> {
        Vec2{x:0.0, y:0.0}
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
        if i < progress/2 {
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

fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
    if value < min {
        return min;
    }
    else if value > max {
        return max;
    }
    value
}

fn divide_roundup(numinator: usize, denominator: usize) -> usize {
    if numinator%denominator == 0 {
        return numinator/denominator;
    }
    numinator/denominator+1
}

fn min<T: PartialOrd>(a: T, b: T) -> T {
    if a < b {
        return a;
    }
    b
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
    offset: Vec2<usize>,
    thread_size: Vec2<usize>,
    color_buffer_size: Vec2<usize>,
    sample_count: usize,
    center: Vec2<Real>,
    view_size: Vec2<Real>,
}

impl ThreadDescryptor {
    fn new() -> ThreadDescryptor {
        ThreadDescryptor {
            offset: Vec2::<usize>::new(),
            thread_size: Vec2::<usize>::new(),
            color_buffer_size: Vec2::<usize>::new(),
            sample_count: 0,
            center: Vec2::<Real>::new(),
            view_size: Vec2::<Real>::new(),
        }
    }
}

fn thread_worker(color_buffer: Arc<Mutex<Vec<Color>>>, desc: ThreadDescryptor, id: usize, finishing_sender: Sender<usize>) {
    let mut temp_color_buffer = vec![Color::new(); desc.thread_size.x * desc.thread_size.y];
    let mut rng = rand::thread_rng();

    //println!("New Thread: x {}, y {}, width {}, height {}", desc.offset.x, desc.offset.y, desc.thread_size.x, desc.thread_size.y);

    // Render the fractal into the temporary color buffer
    for y in 0..desc.thread_size.y {
        for x in 0..desc.thread_size.x {
            let mut pixel_color = Color::new();
            // Stochastic Sampling
            for _ in 0..desc.sample_count {
                let mut norm_pos = Vec2::<Real>::new();
                norm_pos.x = (((x+desc.offset.x) as Real) + rng.gen_range(-0.5, 0.5))/(desc.color_buffer_size.x as Real) * 2.0 - 1.0; // [-1:1]
                norm_pos.y = (((y+desc.offset.y) as Real) + rng.gen_range(-0.5, 0.5))/(desc.color_buffer_size.y as Real) * 2.0 - 1.0; // [-1:1]
                let mut pos = Complex {r: 0.0, i: 0.0};
                pos.r = desc.center.x + norm_pos.x * desc.view_size.x / 2.0; // real axis
                pos.i = desc.center.y + norm_pos.y * desc.view_size.y / 2.0; // imaginary axis
                let mut iterations: u32 = 0;
                let mut temp = Complex {r: 0.0, i: 0.0};
                while temp.length() <= MAX_LENGTH && iterations < MAX_ITERATIONS {
                    temp = temp.squared().add(&pos);
                    iterations += 1;
                }
                pixel_color.add(COLOR_PALETTE[(iterations%16) as usize]);
            }
            pixel_color.divide(desc.sample_count as Real);
            temp_color_buffer[y * desc.thread_size.x + x] = pixel_color;
        }
    }

    // copy the temporary color buffer after locking the color_buffer mutex
    let mut cb = color_buffer.lock().unwrap();
    for y in 0..desc.thread_size.y {
        for x in 0..desc.thread_size.x {
            cb[(y+desc.offset.y) * desc.color_buffer_size.x + (x+desc.offset.x)] = temp_color_buffer[y * desc.thread_size.x + x];
        }
    }

    finishing_sender.send(id).unwrap();
}

fn main() {
    const BUFFER_WIDTH: usize = 1366; // 7680; // 3840; // 1366;
    const BUFFER_HEIGHT: usize = 768; // 4320; // 2160; // 768;
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
    print_progress(0);
    let start_time = time::Instant::now();

    // Fill threads descryptors
    let mut threads_descryptors = Vec::new();
    for y in 0..divide_roundup(BUFFER_HEIGHT, THREAD_HEIGHT) {
        for x in 0..divide_roundup(BUFFER_WIDTH, THREAD_WIDTH) {
            let mut new_desc = ThreadDescryptor::new();
            new_desc.offset = Vec2::<usize>{x: x * THREAD_WIDTH, y: y * THREAD_HEIGHT};
            let max_width = BUFFER_WIDTH - x*THREAD_WIDTH;
            let max_height = BUFFER_HEIGHT - y*THREAD_HEIGHT;
            new_desc.thread_size = Vec2::<usize>{x: clamp(THREAD_WIDTH, 0, max_width-1), y: clamp(THREAD_HEIGHT, 0, max_height-1)};
            new_desc.color_buffer_size = Vec2::<usize>{x: BUFFER_WIDTH, y: BUFFER_HEIGHT};
            new_desc.sample_count = SAMPLE_COUNT;
            new_desc.center = Vec2::<Real>{x: CENTER_X, y:CENTER_Y};
            new_desc.view_size = Vec2::<Real>{x: VIEW_WIDTH, y:VIEW_HEIGHT};
            threads_descryptors.push(new_desc);
        }
    }

    // Spawn threads
    let mut next_thread = 0usize;
    let mut threads = Vec::<thread::JoinHandle<()>>::new();
    let (sender, receiver) = channel::<usize>();
    // Spawn THREAD_COUNT thread first
    for i in 0..min(threads_descryptors.len(), THREAD_COUNT) {
        let descryptor = threads_descryptors[i];
        let color_buffer_clone = color_buffer.clone();
        let sender_clone = sender.clone();
        threads.push(thread::spawn(move || thread_worker(color_buffer_clone, descryptor, i, sender_clone)));
        next_thread += 1;
    }
    let mut finished_threads = 0usize;
    while finished_threads < threads_descryptors.len() {
        let finished_id = receiver.recv().unwrap();
        finished_threads += 1;
        let progress = finished_threads*100/threads_descryptors.len();
        print_progress(progress as u32);
        // print!("{}/{}   ", finished_threads, threads_descryptors.len()); // thread number
        // Spawn a new thread if needed
        if next_thread < threads_descryptors.len() {
            let descryptor = threads_descryptors[next_thread];
            let color_buffer_clone = color_buffer.clone();
            let sender_clone = sender.clone();
            // Replace the finished thread handle
            threads[finished_id] = thread::spawn(move || thread_worker(color_buffer_clone, descryptor, finished_id, sender_clone));
            next_thread += 1;
        }
    }

    // join all threads
    for thread in threads {
        thread.join().unwrap();
    }

    let duration = time::Instant::now().duration_since(start_time).as_secs();
    println!("\nFinished rendering in {}h{}m{}s", (duration/60/60), (duration/60)%60, duration%60);

    let cb = color_buffer.lock().unwrap();
    save_image(&cb, BUFFER_WIDTH, BUFFER_HEIGHT, "output/image.png");
    println!("Saved buffer to image.png");
}

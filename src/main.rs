use std::path::Path;
use std::fs::File;
use std::io::BufWriter;

type Real = f32;

#[derive(Copy, Clone)]
struct Color {
    r: Real,
    g: Real,
    b: Real,
    a: Real,
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

fn main() {
    const WIDTH: usize = 1024;
    const HEIGHT: usize = 1024;
    let mut color_buffer = vec![Color {r:0.0, g:0.0, b:0.0, a:0.0}; WIDTH * HEIGHT]; // Row major

    println!("Drawing the buffer...");
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            //let intensity = (x as Real)/(WIDTH as Real);
            let intensity = (y as Real)/(HEIGHT as Real);
            color_buffer[y * WIDTH + x] = Color {r: intensity, g: intensity, b: intensity, a: 1.0};
        }
    }

    println!("Saving buffer to PNG...");
    save_image(&color_buffer, WIDTH, HEIGHT, "output/image.png");
    println!("Saved buffer to image.png");
}

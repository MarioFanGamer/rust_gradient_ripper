use colour::{HdmaColourMode};

pub mod colour;
mod hdma;

extern crate clap;

use std::{fs::File, io::Write};
use std::path::Path;
use image::open;
use clap::{Arg, App};

const MAX_SCANLINES: u32 = 224; // How large the image can be until the HDMA table must be a

fn main() {
    const OPTIMISE_TABLE: bool = if cfg!(debug_assertions) {false} else {true};

    let matches =
    App::new("HDMA Gradient Ripper")
        .version("1.0")
        .author("MarioFanGamer")
        .about("A small tool which allows you to create an HDMA gradient from an image.")
        .arg(
            Arg::with_name("INPUT")
            .help("The image source to be ripped.")
            .takes_value(true)
        )
        .arg(
            Arg::with_name("xpos")
            .help("The X position of the column to rip (default: 0).")
            .short("x")
            .long("xpos")
            .takes_value(true)
        )
        .arg(
            Arg::with_name("ystart")
            .help("The first Y position of the column to rip (default: 0).")
            .short("s")
            .long("start")
            .takes_value(true)
        )
        .arg(
            Arg::with_name("yend")
            .help("The final Y position to rip (default: height of image).")
            .short("e")
            .long("end")
            .takes_value(true)
        )
        .arg(
            Arg::with_name("output")
            .help("The name of the ASM file.")
            .short("o")
            .long("output")
            .takes_value(true)
        )
        .arg(
            Arg::with_name("height")
            .help("The height of the output table (by default, height of image).")
            .short("h")
            .long("height")
            .takes_value(true)
        )
        .arg(
            Arg::with_name("hdma_mode")
            .help("The mode of the HDMA tables.")
            .short("m")
            .long("mode")
            .takes_value(true)
        )
        .arg(
            Arg::with_name("cgram")
            .help("The colour index in CG-RAM.")
            .short("c")
            .long("cgram")
            .takes_value(true)
        )
        .arg(
            Arg::with_name("verbose")
            .help("Displays the amount of data.")
            .short("v")
            .long("verbose")
            .takes_value(true)
        )
        .get_matches();
    
    // Get the input values
    match matches.value_of("INPUT") {
        Some(x) => {
            let input_name = String::from(x);
            let output_name = String::from(matches.value_of("output").unwrap_or("gradient.asm"));

            // Load up the image since we need its data.
            let image = match open(&input_name) {
                Err(why) => panic!("Couldn't open {}: {}", &input_name, why),
                Ok(x) => x.into_rgb8()
            };
        
            let image_height = image.height();
    
            // Get the command line input for in- and output (or replace them with default values).
            let height = match matches.value_of("height") {
                Some(x) => x.parse().expect("Invalid height!"),
                None => if image_height < MAX_SCANLINES {MAX_SCANLINES} else {image_height}
            };
            let y_start = match matches.value_of("ystart") {
                Some(x) => x.parse().expect("Invalid Y position!"),
                None => 0
            };
            let y_end = match matches.value_of("yend") {
                Some(x) => x.parse().expect("Invalid Y position!"),
                None => image_height
            };
            let x_pos = match matches.value_of("xpos") {
                Some(x) => x.parse().expect("Invalid X position!"),
                None => 0
            };
            let cgram_index = match matches.value_of("cgram") {
                Some(x) => Some(x.parse().expect("Invalid CG-RAM index!")),
                None => None
            };

            let mode = match matches.value_of("hdma_mode").unwrap_or("a") {
                "s" | "single" => HdmaColourMode::FixedClourThree,
                "d" | "double" => HdmaColourMode::FixedClourTwo,
                "b" | "big" => HdmaColourMode::BigGradient,
                "c" | "cgram" => HdmaColourMode::CgRam,
                "a" | "auto" => if height > 224 {HdmaColourMode::BigGradient} else {HdmaColourMode::FixedClourTwo},
                _ => panic!("The entered option is invalid!")
            };
        
            // Handle errors (invalid inputs)
            if y_start > image_height || y_end > image_height {
                panic!("The entered Y position is located outside of the image!");
            }
            if x_pos > image.width() {
                panic!("The entered X position is located outside of the image!")
            }

            // Handle warnings (questionable inputs)
            if height < MAX_SCANLINES {
                let warning = format!("Warning: The output height you entered is {} which is smaller than than {max}.
                I recommend you to use a height of at least {max}.",
                height, max = MAX_SCANLINES);
                eprintln!("{}", warning);
            }

            if (mode != HdmaColourMode::BigGradient) & (height > MAX_SCANLINES) {
                let warning = format!("Warning: The image height you entered is {} which is larger than {}.
                I recommend you to use a big gradient instead.",
                height, MAX_SCANLINES);
                eprintln!("{}", warning);
            }

            let output_path = Path::new(&output_name);

            let output_data = colour::write_table(height, x_pos, y_start, y_end, mode, cgram_index, image, OPTIMISE_TABLE);
            write_file(output_data, output_path)
        },
        None => {
            let mut input_name = String::new();

            println!("Rust Gradient Ripper\n");

            print!("Enter the image to be ripped: ");

            std::io::stdout().flush().unwrap();

            std::io::stdin().read_line(&mut input_name).expect("Error: Couldn't read input.");

            let input_name = input_name.trim();

            // Load up the image since we need its data.
            let image = match open(input_name) {
                Err(why) => panic!("Couldn't open {}: {}", input_name, why),
                Ok(x) => x.into_rgb8()
            };

            print!("Enter the the name of the ASM file: ");

            std::io::stdout().flush().unwrap();

            let mut output_name = String::new();

            std::io::stdin().read_line(&mut output_name).expect("Error: Couldn't read input.");
        
            let image_height = image.height();

            // Get additional data
            let height = if image_height < MAX_SCANLINES {MAX_SCANLINES} else {image_height};
            let mode = if height > MAX_SCANLINES {HdmaColourMode::BigGradient} else {HdmaColourMode::FixedClourTwo};
            let x_pos = 0;
            let y_start = 0;
            let y_end = image_height;

            if cfg!(debug_assertions) {
                println!("Image height: {}", image_height);
                println!("Output height: {}", height);
                println!("Input X position: {}", x_pos);
                println!("Input Y position start: {}", y_start);
                println!("Input Y position end: {}", y_end);
                println!("Input height: {}", y_end - y_start);
                match mode {
                    HdmaColourMode::FixedClourTwo => println!("Fixed colour, two tables."),
                    HdmaColourMode::BigGradient => println!("Fixed colour, one big table"),
                    _ => println!("Some other table (shouldn't ever happen here).")
                }
            }

            let output_name = output_name.trim();

            let output_path = Path::new(&output_name);

            let output_data = colour::write_table(height, x_pos, y_start, y_end, mode, None, image, OPTIMISE_TABLE);

            write_file(output_data, output_path);
        }
    }

    fn write_file(text_data: String, output_path: &Path) {

        // Load the path
        let display = output_path.display();
    
        let mut file = match File::create(&output_path) {
            Err(why) => panic!("Couldn't create {}, {}", display, why),
            Ok(file) => file,
        };

        match file.write_all(text_data.as_bytes()) {
            Err(why) => panic!("Couldn't write to {}, {}", display, why),
            Ok(_) => println!("HDMA table successfully generated!"),
        }
    }
}

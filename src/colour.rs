use crate::hdma::{HdmaTable, HdmaRow, HdmaWriteMode};

use image::{Rgb, ImageBuffer};

#[derive(Copy, Clone, PartialEq)]
pub enum HdmaColourMode {
    FixedClourThree,
    FixedClourTwo,
    BigGradient,
    CgRam,
}

// The colour indeces of the colours
// Uses RGB values
#[derive(Copy, Clone)]
enum ColourIndex {
    Red = 0,
    Green = 1,
    Blue = 2
}

impl ColourIndex {
    // Gets the colour bit for fixed colour.
    fn colour_bit(&self) -> u8 {
        match self {
            ColourIndex::Red => 0x20,
            ColourIndex::Green => 0x40,
            ColourIndex::Blue => 0x80
        }
    }
}

// Note for both functions:
// SNES colour is 15-bit BGR while we use 24-bit RGB or 5-bit colours and 8-bit colours.
// This basically means that in order to get an 8-bit colour, you just divide the value by 8 (three right shifts).
// However, both 255_8 and 31_5 result in the same colour so you can't just divide the value by 8, right?
// That's what I thought at first as well and I multiplied the value by 31/255.
// However, according to BSNES, the lowest three bits are copies of bits 2, 3 and 4.
// It's basically this formula: ---43210 -> 43210432
// Ultimately, I can just divide the value by 8 because it's just 5-bit colour multiplied by 8 plus some constant.

fn to_fixed_colour(colour: Rgb<u8>, colour_index: ColourIndex) -> u8 {
    return (colour.0[colour_index as usize] & !0x07) / 8 + colour_index.colour_bit();
}

fn to_cgram_colour(colour: Rgb<u8>) -> u16 {
    let red = (colour.0[ColourIndex::Red as usize] & !0x07) >> 3;
    let green = (colour.0[ColourIndex::Green as usize] & !0x07) >> 3;
    let blue = (colour.0[ColourIndex::Blue as usize] & !0x07) >> 3;

    return ((red as u16) << 0) | ((green as u16) << 5) | ((blue as u16) << 10);
}

pub fn get_rgb_from_image(image: ImageBuffer<Rgb<u8>, Vec<u8>>, x_input: u32, y_start: u32, y_end: u32, output_height: u32) -> Vec<Rgb<u8>> {
    let mut colours = Vec::new();

    // Calculate the transformation of the rows.
    let input_height = y_end - y_start;
    let mut y_real: f64 = y_start as f64;
    let delta_y: f64 = output_height as f64 / input_height as f64;

    // Get the colour of the very left pixel
    for _ in 0..output_height {
        let colour = *image.get_pixel(x_input, y_real.round() as u32);

        y_real += delta_y;

        colours.push(colour);
    }

    return colours;
}

// A three colour version of the above.
pub fn create_mode_0_tables(colours: Vec<Rgb<u8>>) -> [HdmaTable; 3] {
    let mut red_table = HdmaTable::new_real_table (Vec::new(), 1, HdmaWriteMode::Bytes, "red_table");
    let mut green_table = HdmaTable::new_real_table (Vec::new(), 1, HdmaWriteMode::Bytes, "green_table");
    let mut blue_table = HdmaTable::new_real_table (Vec::new(), 1, HdmaWriteMode::Bytes, "blue_table");

    for colour in colours {
        // Store colours individually because it's easier to read that way
        let red = to_fixed_colour(colour, ColourIndex::Red);
        let green = to_fixed_colour(colour, ColourIndex::Green);
        let blue = to_fixed_colour(colour, ColourIndex::Blue);

        red_table.push(HdmaRow::new_scanline(&[red]));
        green_table.push(HdmaRow::new_scanline(&[green]));
        blue_table.push(HdmaRow::new_scanline(&[blue]));
    }

    return [red_table, green_table, blue_table];
}

// Creates two tables, a single colour table and a dual coloured table.
// Optimised colours are chosen.
pub fn create_mode_2_table(colours: Vec<Rgb<u8>>) -> [HdmaTable; 2] {
    let mut colour_count = get_colour_count(&colours);

    // Sort colour by colour count
    colour_count.sort_by_key(|x| x.0);

    // Hack (would have preferred it with an inline if but for some reason, it spits out a "()" as a type).
    let single_colour;

    // The idea is to merge only two tables together whichever have the closest amount of colour changes.
    // Basically: If one colour has got only 7 rows, the other two 23 and 34, it's the most efficient
    // to combine the latter two than the former with any of the latter.
    if (colour_count[0].0 - colour_count[1].0).abs() > (colour_count[1].0 - colour_count[2].0).abs() {
        single_colour = colour_count[0].1;
    }
    else {
        single_colour = colour_count[2].1;
    };

    // Set the hdma tables as well as the other two colours depending on the value for single table.
    let (mut single_table, mut dual_table, dual_colour_1, dual_colour_2) =
    match single_colour {
        ColourIndex::Red => {
            (
                HdmaTable::new_real_table(Vec::new(), 1, HdmaWriteMode::Bytes, "red_table"),
                HdmaTable::new_real_table(Vec::new(), 2, HdmaWriteMode::Bytes, "green_blue_table"),
                ColourIndex::Green,
                ColourIndex::Blue
            )
        }
        ColourIndex::Green => {
            (
                HdmaTable::new_real_table(Vec::new(), 1, HdmaWriteMode::Bytes, "green_table"),
                HdmaTable::new_real_table(Vec::new(), 2, HdmaWriteMode::Bytes, "red_blue_table"),
                ColourIndex::Red,
                ColourIndex::Blue
            )
        }
        ColourIndex::Blue => {
            (
                HdmaTable::new_real_table(Vec::new(), 1, HdmaWriteMode::Bytes, "blue_table"),
                HdmaTable::new_real_table(Vec::new(), 2, HdmaWriteMode::Bytes, "red_green_table"),
                ColourIndex::Red,
                ColourIndex::Green
            )
        }
    };

    // Now write the colours to the HDMA table.
    for colour in colours {
        // Store colours individually because it's easier to read that way
        let single = to_fixed_colour(colour, single_colour);
        let dual_1 = to_fixed_colour(colour, dual_colour_1);
        let dual_2 = to_fixed_colour(colour, dual_colour_2);

        single_table.push(HdmaRow::new_scanline(&[single]));
        dual_table.push(HdmaRow::new_scanline(&[dual_1, dual_2]));
    }

    return [single_table, dual_table];
}

// A three colour version of the above.
pub fn create_big_gradient_table(colours: Vec<Rgb<u8>>) -> HdmaTable {
    let mut output = HdmaTable::new(Vec::new(), 3, HdmaWriteMode::Bytes, "gradient_table", 0xFF);

    for colour in colours {
        // Store colours individually because it's easier to read that way
        let red = to_fixed_colour(colour, ColourIndex::Red);
        let green = to_fixed_colour(colour, ColourIndex::Green);
        let blue = to_fixed_colour(colour, ColourIndex::Blue);

        output.push(HdmaRow::new_scanline(&[red, green, blue]));
    }

    return output;
}

pub fn create_cgram_table(colours: Vec<Rgb<u8>>, cgram_index: Option<u8>) -> HdmaTable {
    let row_size = match cgram_index {
        Some(_) => 4,
        None => 2
    };

    let mut output = HdmaTable::new_real_table(Vec::new(), row_size, HdmaWriteMode::Words, "colour_table");

    for colour in colours {
        let cgram_colour = to_cgram_colour(colour);
        let low_byte = (cgram_colour & 0x00FF) as u8;
        let high_byte = ((cgram_colour & 0xFF00) >> 9) as u8;

        match cgram_index {
            Some(index) => output.push(HdmaRow::new_scanline(&[0x00, index, low_byte, high_byte])),
            None => output.push(HdmaRow::new_scanline(&[low_byte, high_byte])),
        }
    }

    return output;
}

fn get_colour_triplet(colour: Rgb<u8>) -> (u8, u8, u8) {
    let red = colour.0[ColourIndex::Red as usize];
    let green = colour.0[ColourIndex::Green as usize];
    let blue = colour.0[ColourIndex::Blue as usize];
    return (red, green, blue);
}

// Counts the amount of colours changes in the table
// This is necessary for finding the most efficient mode 2 table
fn get_colour_count(colours: &Vec<Rgb<u8>>) -> [(isize, ColourIndex); 3] {
    // Set counters to 1 since there already is a change
    let mut counter_red = (1, ColourIndex::Red);
    let mut counter_green = (1, ColourIndex::Green);
    let mut counter_blue = (1, ColourIndex::Blue);

    let mut last_red: Option<u8> = None;
    let mut last_green: Option<u8> = None;
    let mut last_blue: Option<u8> = None;

    // Now count how often the colours change
    for colour in colours {
        let (current_red, current_green, current_blue) = get_colour_triplet(*colour);

        // Basically: If last_red has value and is equal to current_red,
        // increment the count.
        // Otherwise (last colour has not value) or colours are different,
        // load the next colour
        // Do that for red, green and blue
        match (last_red, current_red) {
            (Some(last), current) if last == current => {
                counter_red.0 += 1;
            }
            _ => {
                last_red = Some(current_red);
            }
        }
        match (last_green, current_red) {
            (Some(last), current) if last == current => {
                counter_green.0 += 1;
            }
            _ => {
                last_green = Some(current_green);
            }
        }
        match (last_blue, current_red) {
            (Some(last), current) if last == current => {
                counter_blue.0 += 1;
            }
            _ => {
                last_blue = Some(current_blue);
            }
        }
    };

    return [counter_red, counter_green, counter_blue];
}



// That one creates a string from the ASM file.
pub fn write_table(height: u32, x_pos: u32, y_start: u32, y_end: u32, mode: HdmaColourMode,
    cgram_index: Option<u8>, image: ImageBuffer<Rgb<u8>, Vec<u8>>, optimise: bool) -> String {

    match mode {
        HdmaColourMode::FixedClourThree => {
            let colours = get_rgb_from_image(image, x_pos, y_start, y_end, height);
            let hdma_tables = create_mode_0_tables(colours);

            let mut output = String::new();

            for mut table in hdma_tables {
                if optimise {
                    table.coagulate();
                }
                output.push_str(&format!("{}\n", table.write_table()));
            }

            return output;
        }
        HdmaColourMode::FixedClourTwo => {
            let colours = get_rgb_from_image(image, x_pos, y_start, y_end, height);
            let hdma_tables = create_mode_2_table(colours);

            let mut output = String::new();

            for mut table in hdma_tables {
                if optimise {
                    table.coagulate();
                }
                output.push_str(&format!("{}\n", table.write_table()));
            }

            return output;
        }
        HdmaColourMode::BigGradient => {
            let colours = get_rgb_from_image(image, x_pos, y_start, y_end, height);
            let mut hdma_table = create_big_gradient_table(colours);

            if optimise {
                hdma_table.coagulate_repeat();
            }

            return hdma_table.write_table();
        }
        HdmaColourMode::CgRam => {
            let colours = get_rgb_from_image(image, x_pos, y_start, y_end, height);
            let mut hdma_table = create_cgram_table(colours, cgram_index);

            if optimise {
                hdma_table.coagulate();
            }

            return hdma_table.write_table();
        }
    }
}

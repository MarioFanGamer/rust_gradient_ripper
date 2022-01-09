#[derive(Copy, Clone)]
pub enum HdmaWriteMode {
    Bytes,
    Words
}

// A rust implementation of the
// Types can be either repeat, which means same value for X scanlines.
// Or continuous which changes the value every scanline and is a vector of the data (scanline count is implied here).
// In all cases, the data is an array of four u8 bytes since this is how much data you can send with HDMA.
pub enum HdmaRow {
    Repeat { count: usize, data: [u8; 4]},
    Continuous { data: Vec<[u8; 4]>},
    Finish
}

impl HdmaRow {
    // Generates a new repeat row.
    pub fn new_repeat(count: usize, data: &[u8]) -> Self {
        let mut data_array: [u8; 4] = [0; 4];   // Initialises a four byte array with zeroes
        for (array_byte, vec_byte) in data_array.iter_mut().zip(data.iter()) {
            *array_byte = *vec_byte;
        }
        return HdmaRow::Repeat { count: count, data: data_array };
    }
    
    // Generates a new continuous row.
    // It takes the size as an argument to avoid padding bytes.
    pub fn new_continuous(data: &[u8], data_size: u32) -> Self {
        // Prevent invalid block sizes.
        if (data_size < 1) | (data_size > 4) {
            panic!("Error: The specified row count is outside of the range (must be between 1 and including 4).");
        }
        // Round up the length of the vector to the nearest multiple of data_size.
        let row_count = (data.len() as f64 / data_size as f64).ceil() as usize;

        // Generate a new vector of arrays of four u8 bytes.
        let mut row_data: Vec<[u8; 4]> = Vec::new();

        // Get iterator of the data.
        let mut data_iterator = data.iter();
        
        // Now things get complicated: The resulting data is always a multiple of four
        // but data may not be it. To fix this, I use wrote the following code to basically
        // add in padding bytes into the original data. It simply means: Loop the code
        // size/n (rounded up) times, run an inner loop which takes n bytes of the original data
        // and store it to the current array.
        // This is why I use an iterator: It easily allows me to get the next byte in the original data
        // AND use default values if the data runs out.
        for _ in 0..row_count {
            let mut current_data = [0, 0, 0, 0];

            for i in 0..data_size {
                current_data[i as usize] = *data_iterator.next().unwrap_or(&0);
            }

            row_data.push(current_data);
        }

        return HdmaRow::Continuous { data: row_data };
    }
    
    // Generates a repeat row of a single scanline.
    pub fn new_scanline(data: &[u8]) -> Self {
        return Self::new_repeat(1, data);
    }
}

// This is basically how an HDMA table on the SNES looks like.
// There is an array of vectors which contains n bytes.
// Liberties are taken for row_size which can also accept 3
// (normally, it's only 1, 2 or 4 bytes), total row count (to
// handle pseudo-tables like big gradients), write mode and
// table name (display in the ASM file only).
pub struct HdmaTable {
    rows: Vec<HdmaRow>,
    row_size: usize,
    max_row_count: usize,
    write_mode: HdmaWriteMode,
    table_name: &'static str
}

impl HdmaTable {
    // Generic table
    pub fn new(rows: Vec<HdmaRow>, row_size: usize, write_mode: HdmaWriteMode, table_name: &'static str, max_row_count: usize) -> Self {
        if (row_size < 1) | (row_size > 4) {
            panic!("Error: The specified row count is outside of the range.");
        }
        Self { rows: rows, row_size: row_size, table_name: table_name, write_mode: write_mode, max_row_count: max_row_count }
    }

    // An actual HDMA table, total row count is limited to 0x80 lines.
    pub fn new_real_table(rows: Vec<HdmaRow>, row_size: usize, write_mode: HdmaWriteMode, table_name: &'static str) -> Self {
        if (row_size < 1) | (row_size > 4) {
            panic!("Error: The specified row count is outside of the range.");
        }
        Self { rows: rows, row_size: row_size, table_name: table_name, write_mode: write_mode, max_row_count: Self::MAX_REP_ROWS }
    }

    // Adds a new HDMA row to the table.
    pub fn push(self: &mut Self, row: HdmaRow) {
        self.rows.push(row);
    }

    // How many rows can exist until we need to break?
    // This is different for repeating and continuous rows.
    const MAX_REP_ROWS: usize = 0x80;
    const MAX_CONT_ROWS: usize = 0x7F;

    // The constant we need to OR to enable continuous mode.
    const CONT_BIT: usize = 0x80;

    // Thanks for Selicre for this routine! I just made some minor changes and added comments.
    pub fn coagulate(self: &mut Self) {
        let hdma_table = &mut self.rows;

        let old = std::mem::take(hdma_table);
        
        for mut i in old.into_iter() {
            // Basically: Get the last value from the hdma_table.
            // If one doesn't exist, push the current value to the stack.
            let mut last = if let Some(c) = hdma_table.last_mut() {
                c
            } else {
                hdma_table.push(i);
                continue;
            };
            // If continuous HDMA is allowed (i.e. in real HDMA tables),
            // Do these various checks.
            // Though realistically, only row counts of 1 exists in this table,
            // it can work with any form of the table.
            match (&mut last, &mut i) {
                // If two colours are identical regardless of row count,
                // it's a repeating table
                (
                    HdmaRow::Repeat { count: rows_a, data: old },
                    HdmaRow::Repeat { count: rows_b, data: new },
                ) if old == new => {
                    *last = HdmaRow::Repeat { count: *rows_a + *rows_b, data: *new };
                }
                // On the other hand, if there are two single rows with different colours,
                // it's a continuous table.
                (
                    HdmaRow::Repeat { count: 1, data: old },
                    HdmaRow::Repeat { count: 1, data: new },
                ) if old != new => {
                    *last = HdmaRow::Continuous { data: vec![*old, *new] };
                },
                // If the row count of the previous row is 1 and followed by a continuous row,
                // append it at the beginning of the continuous rows.
                (
                    HdmaRow::Repeat { count: 1, data: old },
                    HdmaRow::Continuous { data },
                ) => {
                    data.insert(0, *old);
                },
                // The opposite of the above: Continuous row is followed by a single row of repeat?
                // If the data is different, append the latter to the former!
                // In addition, I edited that one, because...
                (  
                    HdmaRow::Continuous { data },
                    HdmaRow::Repeat { count: 1, data: new },
                ) => {
                    // This fixes a feedback loop where once a continuous table has been generated,
                    // it stays so even though it makes more sense to write a repeat there.
                    let last_row = &mut match data.pop() {
                        Some(x) => x,
                        None => panic!("Something has gone wrong in the evaluation of the table!")
                    };
                    // If both tables are equal, write a new repeat row with the same data.
                    // No need to fix single continuous rows because these are identical in function,
                    // though if enough people complain, maybe I'll fix it?
                    if last_row == new {
                        hdma_table.push( HdmaRow::Repeat { count: 2, data: *last_row } );
                    }
                    // Push both rows back.
                    else {
                        data.push(*last_row);
                        data.push(*new);
                    }
                },
                // Merge two continuous tables, it makes no sense otherwise (unless both values are equal, of course).
                (  
                    HdmaRow::Continuous { data },
                    HdmaRow::Continuous { data: new },
                ) => {
                    data.append(new);
                },
                // Add the table to hdma_table otherwise.
                _ => {
                    hdma_table.push(i);
                }
            }
        }

        // The final row can be as small as 1 since by that point, HDMA is finished and
        // drawing any more rows (if row count is greater than 0x80) is just a waste of space.
        let last_row = hdma_table.last_mut();

        // If a repeat table, modify last row to 
        match last_row {
            Some(row) => {
                match row {
                    HdmaRow::Repeat { count, data: _ } => {
                        *count = 1;
                    }
                    _ => {}
                }
            }
            None => {
                //Err("Couldn't load the last row for some reason.");
            }
        }

        // Append a zero to the table as it is the termination byte.
        hdma_table.push(HdmaRow::Finish);
    }

    // That one is just the coagulate routine stripped down to only handle repeating tables.
    pub fn coagulate_repeat(self: &mut Self) {
        let hdma_table = &mut self.rows;

        let old = std::mem::take(hdma_table);
        
        for mut i in old.into_iter() {
            // Basically: Get the last value from the hdma_table.
            // If one doesn't exist, push the current value to the stack.
            let mut last = if let Some(c) = hdma_table.last_mut() {
                c
            } else {
                hdma_table.push(i);
                continue;
            };
            // Coalgulate for only row count.
            // Use for scrollable tables which aren't real HDMA tables and thus can be as large
            // as 0xFF rows.
            match (&mut last, &mut i) {
                (
                    HdmaRow::Repeat { count: rows_a, data: old },
                    HdmaRow::Repeat { count: rows_b, data: new },
                ) if old == new => {
                    *last = HdmaRow::Repeat { count: *rows_a + *rows_b, data: *new };
                }
                _ => {
                    hdma_table.push(i);
                }
            }
        }

        // Append a zero to the table as it is the termination byte.
        hdma_table.push(HdmaRow::Finish);
    }

    // Write the HDMA table.
    // Do note that the actually written HDMA table.
    pub fn write_table(self: Self) -> String {
        // Put the table name first
        let mut output = String::from(format!("{}:\n", &self.table_name));

        output.push_str(&match self.write_mode {
            HdmaWriteMode::Bytes => self.write_bytes(),
            HdmaWriteMode::Words => self.write_words()
        });

        return output;
    }

    fn write_bytes(self: Self) -> String {
        let mut output = String::new();

        for row in self.rows {
            match row {
                // Repeat:
                // db $xx : db $yy
                // Where xx is the row count (may not exceed the total row count) and
                // yy the data (can be up to four bytes)
                HdmaRow::Repeat { mut count, data } => {
                    loop {
                        // Write down the scanline count
                        // But it cannot exeed more than 0x80.
                        output.push_str(&format!("db ${:02X}",
                            if count < self.max_row_count {count} else {self.max_row_count}));
                        // Write down the bytes (right now, only single bytes)
                        for i in 0..self.row_size {
                            output.push_str(&format!(",${:02X}", data[i]));
                        }
                        output.push('\n');
                        // If there are less than the total row count left
                        if count < self.max_row_count {
                            break;
                        }
                        // Otherwise subtract the remaining row count from the max row count.
                        count = count - self.max_row_count;
                    }
                }
                // Repeat:
                // db $xx : db $yy,$zz...
                // Where xx is the row count (between 0x81 and 0xFF),
                // followed by xx-0x80 units of data
                // Note that continuous lacks any length information since that's provided by the vector.
                HdmaRow::Continuous { data } => {
                    // How many rows there are to write,
                    // the current index
                    // and an iterator
                    // That loop is quite complicated
                    let mut total_rows = data.len();
                    let mut current_byte = 0;
                    let mut row_iterator = data.iter();
                    loop {
                        // Put down the first byte, the scanline count
                        current_byte += 1;
                        if current_byte == 1 {
                            if total_rows > Self::MAX_CONT_ROWS {
                                output.push_str("db $FF");
                                total_rows -= Self::MAX_CONT_ROWS;
                            }
                            else {
                                output.push_str(&format!("db ${:02X}", total_rows + Self::CONT_BIT));
                            }
                            continue;
                        }
                        // If more than 0x80 units of data (including scanline count) have been written
                        // Put down a new line
                        else if current_byte > (Self::MAX_CONT_ROWS + 1) {
                            output.push('\n');
                            current_byte = 0;
                            continue;
                        }
                        // Otherwise write down so many bytes until the end is reached
                        match row_iterator.next() {
                            Some(x) => {
                                for i in 0..self.row_size {
                                    output.push_str(&format!(",${:02X}", x[i]));
                                }
                            }
                            None => {
                                output.push('\n');
                                break;
                            }
                        }
                    }
                }
                HdmaRow::Finish => {
                    output.push_str("db $00\n")
                }
            }
        }
    
        return output;
    }

    fn write_words(self: Self) -> String {
        let mut output = String::new();

        for row in self.rows {
            match row {
                // Repeat:
                // db $xx : dw $zzyy
                // Where xx is the row count (may not exceed the total row count) and
                // zzyy the data (can be up to four bytes)
                HdmaRow::Repeat { mut count, data } => {
                    loop {
                        // Write down the scanline count
                        // But it cannot exeed more than 0x80.
                        output.push_str(&format!("db ${:02X}",
                            if count < self.max_row_count {count} else {self.max_row_count}));
                        
                        // This gets a bit complex.
                        // Effectively, there are only two cases: Larger than 2 or at most 2.
                        // This is why I use the method of using an if-else-condition whereas
                        // a for-loop would become just too ugly.
                        // Of course, for row_size != {2, 4}, just don't write more to data
                        // that you actually have to.
                        if self.row_size <= 2 {
                            output.push_str(&format!(" : dw ${:02X}{:02X}",
                                data[1], data[0]));
                        }
                        else {
                            output.push_str(&format!(" : dw ${:02X}{:02X},${:02X}{:02X}",
                            data[1], data[0], data[3], data[2]));
                        }

                        output.push('\n');
                        // If there are less than the total row count left
                        if count < self.max_row_count {
                            break;
                        }
                        // Otherwise subtract the remaining row count from the max row count.
                        count = count - self.max_row_count;
                    }
                }
                // Repeat:
                // db $xx : db $zzyy,$wwvv...
                // Where xx is the row count (between 0x81 and 0xFF),
                // followed by xx-0x80 units of data
                // Note that continuous lacks any length information since that's provided by the vector.
                // So we have to use a different solution for that.
                HdmaRow::Continuous { data } => {
                    // How many rows there are to write,
                    // the current index
                    // and an iterator
                    // That loop is quite complicated
                    let mut total_rows = data.len();
                    let mut current_byte = 0;
                    let mut row_iterator = data.iter();
                    loop {
                        // Put down the first byte, the scanline count
                        current_byte += 1;
                        if current_byte == 1 {
                            if total_rows > Self::MAX_CONT_ROWS {
                                output.push_str("db $FF");
                                total_rows -= Self::MAX_CONT_ROWS;
                            }
                            else {
                                output.push_str(&format!("db ${:02X}", total_rows + Self::CONT_BIT));
                            }
                            continue;
                        }
                        // If more than 0x80 units of data (including scanline count) have been written
                        // Put down a new line
                        else if current_byte > (Self::MAX_CONT_ROWS + 1) {
                            output.push('\n');
                            current_byte = 0;
                            continue;
                        }
                        // Otherwise write down so many bytes until the end is reached
                        match row_iterator.next() {
                            Some(data) => {
                                if self.row_size <= 2 {
                                    output.push_str(&format!(" : dw ${:02X}{:02X}",
                                        data[1], data[0]));
                                }
                                else {
                                    output.push_str(&format!(" : dw ${:02X}{:02X},${:02X}{:02X}",
                                    data[1], data[0], data[3], data[2]));
                                }
                            }
                            None => {
                                output.push('\n');
                                break;
                            }
                        }
                    }
                }
                HdmaRow::Finish => {
                    output.push_str("db $00\n")
                }
            }
        }
    
        return output;
    }
}

HDMA GRADIENT RIPPER v0.9
=========================

This piece of software is an HDMA gradient ripper. It allows you to turn an image into an HDMA table for colours of various types (see below).

Usage
-----
This tool works with a command line: Open up your command line (e.g. bash or PowerShell) and type

<code>
gradient_ripper [-s Y_START] [-e Y_END] [-h HEIGHT] [-x XPOS] [-m MODE] [-c CGRAM_IND] [-o OUTPUT] INPUT
</code>

There are a lot of options so let's talk about every single of them:
* INPUT is the most obvious and important one: It's the filename of the gradient input.
* OUTPUT is the filename of the ASM file. It defaults to 
* MODE is the type of table. It can take the following modes (with the first letter as shorthand option):
 - `single` creates three HDMA tables, each with one colour. It is the most space efficient one but uses up three HDMA channels as a result.
 - `double` creates two HDMA tables, one with one and one with two colours. Though the result is generally larger than three single tables, only two HDMA channels are really used. Furthermore, the table uses the most optimised colour output.
 - `big` creates one pseudo-HDMA table. The table is the least optimised one but is primarily used as a compressed scrollable gradient table.
 - `cgram` creates a table which isn't for fixed colour writes but rather for CG-RAM.
 - `auto` is the default option. What it does is to use `double` if the output height is at most 224 scanlines and `big` if larger.
* XPOS is the x position of the image. Valid values is in the range from 0 (default value) to the image width.
* HEIGHT is the height of the output, by default 224 or the image source height depending on which one is larger. The height must be at least 0 but the program will thrown a warning if the height is smaller than the normal scanline count (224 scanlines) or larger than the normal scanline count if it isn't a big gradient (sorry, overscan mode).
* Y_START is the start point of the input. By default, it is 0.
* Y_END is the end point of the input. By default, it is the height of the source image. The height will be scaled for the output.
* CGRAM_IND is the CG-RAM index (and necessary). It is not used if writing to CG-RAM isn't used.

You can also run the tool without any input. In this case, it asks you for the in- and output of the gradient.

Note that for the first non-beta release, some of the options may change (in particular, the Y positions).


Including the HDMA tables
-------------------------
In order to *use* these HDMA tables, you need to run a code. As a result, I included the file `hdma_macros.asm` as a simple way to run HDMA code.
These macros take the inputs channel number and table address (which can be the label) for each table. The channel number must range from 0 to 7 for the SNES in general and specifically for SMW, from 3 to 6 (7 if you use SA-1 Pack).

Here is an example on how it works:
<code>
%hdma_three_channels(3, "red_table", 4, "green_table", 5, "blue_table")
</code>

For two tables, remember that the name isn't fixed. In that case, simply enter the name for the first table followed by the name of the second table (e.g. if red is the single channel then it's "red_table" first and then "green_blue_table").
The order is required since the second table is hardcoded to be the dual colour table.

`hdma_macros.asm` can be included in e.g. `macro_codes.asm` of UberASM Tool.

Note that big gradients are only supported by ["Scrollable" HDMA Gradients](https://www.smwcentral.net/?p=section&a=details&id=23789)) whereas CG-RAM without index isn't supported as they are.


For Developers
--------------
The tool is made of three files: `main.rs`, the user interface of the tool, `hdma.rs` and `colour.rs`.

The purpose of `hdma.rs` is to have a general library for handling HDMA tables. That means, I provide codes for handling HDMA rows and HDMA tables.

For the rows, there is the enum `HdmaRow` where you have the following option:
* You can define HDMA rows which are either repeating (same value for X scanlines), continuous (each scanline uses a different value) or the termination byte.
* Repeating tables (`HdmaRow::Repeat`) have got a scanline count as well as an array with four u8 bytes.
* Continuous tables (`HdmaRow::Continuous`) also an array of four u8 bytes but these are packed in a vector. A side effect is that the line count is implied with the vector size.
* The termination row (`HdmaRow::Finish`) is used for writing the table to mark where to write a "db $00".
* There are three functions provided for creating new rows.
* You can create a new repeating table, which is straightforward, or you can create a continuous table.
* The continuous table maps a vector of u8 to a vector of four byte arrays of u8. It takes in the size. The data in question will be padded so no byte gets skipped.
* New rows can be creates with `new_scanline` which loads an HDMA row with a single scanline of the type continuous.


The other important element is the struct `HdmaTable`. It's supposed to be somewhat a recreation of a real HDMA table as a rust struct with some liberties taken:
* The struct itself contains the data for the HDMA rows (see above), bytes to write (must be between 1 and including 4), total scanline count, write mode and table name.
* Bytes to write is how many bytes there can be per HDMA row (row count excluded). Valid values are 1, 2 and 4 as real tables and 3 also for pseudo-tables.
* Row count is used to determine, how many scanlines you can have. This is primarily done so to determine (note that this may get removed in a later version and be replaced with a function to split large HDMA rows).
* The write mode can be either `HdmaWriteMode::Bytes`, which writes bytes of data only, or `HdmaWriteMode::Words`, which writes one or two words of data. Bytes to write gets rounded up to the nearest multiple of two.
* The table data can be optimised with the routines `coagulate` (allows for both, repeating and continuous tables) and `coagulate_repeat` (handles only repeating tables).


Colour manipulation, on the other hand, is found at `colours.rs`.
* It provides the enum `HdmaColourMode` which generates the type of HDMA table
* There also is the enum `ColourIndex` which is used to hold constants of the colour index (for the Rgb struct) as well as colour bit for fixed colour HDMA.
* `get_rgb_from_image` gets the colours of the HDMA table but don't transform them.
* `create_mode_0_tables`, `create_mode_2_tables`, `create_big_gradient_table` and `create_cgram_table` all generate the corresponding HDMA tables.
* `create_mode_2_tables` in addition optimises the input in these
* `create_cgram_table` also takes a CG-RAM index as a parameter.
* `write_table` takes the image (x position, y range) and table (height, type) data for input and spits out a string.


Keep in mind that all of that is WIP and may be subjected to change. In particular, `main.rs` will be rewritten at some point to remove as much redundancy as possible and the Y position may be merged together (see above).
It also is somewhat untested such as is the case with the lack of any unit tests and other tests.

Furthermore, this is still in development and public API may be subject to change.


Known Bugs
----------
 * Right now, the code to write a large repeat row (127 different scanlines) is broken and I'll have to fix it at some point.
 * Two rows of single scanline continuous rows coagulated as continuous even if both are the same data.
 * Coagulate doesn't work with the termination row right now.
 * Since it still is in development, there may be more bugs I didn't catch.

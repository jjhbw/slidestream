//! Misc utility definitions

use byteorder::ByteOrder;
use failure::{Error};
use image::{Rgba, RgbaImage};

use std::fmt::{Debug};

/// A list of supported formats
///
/// Information gathered from [https://openslide.org/formats/](https://openslide.org/formats/)
///
#[derive(Clone, Debug)]
pub enum Format {
    /// Single-file pyramidal tiled TIFF, with non-standard metadata and compression.
    ///
    /// File extensions:
    ///     .svs, .tif
    Aperio,
    /// Multi-file JPEG/NGR with proprietary metadata and index file formats, and single-file
    /// TIFF-like format with proprietary metadata.
    ///
    /// File extensions:
    ///     .vms, .vmu, .ndpi
    Hamamatsu,
    /// Single-file pyramidal tiled BigTIFF with non-standard metadata.
    ///
    /// File extensions
    ///     .scn
    Leica,
    /// Multi-file with very complicated proprietary metadata and indexes.
    ///
    /// File extensions
    ///     .mrxs
    Mirax,
    /// Single-file pyramidal tiled TIFF or BigTIFF with non-standard metadata.
    ///
    /// File extensions
    ///     .tiff
    Phillips,
    /// SQLite database containing pyramid tiles and metadata.
    ///
    /// File extensions
    ///     .svslide
    Sakura,
    /// Single-file pyramidal tiled TIFF, with non-standard metadata and overlaps. Additional files
    /// contain more metadata and detailed overlap info.
    ///
    /// File extensions
    ///     .tif
    Trestle,
    /// Single-file pyramidal tiled BigTIFF, with non-standard metadata and overlaps.
    ///
    /// File extensions
    ///     .bif, .tif
    Ventana,
    /// Single-file pyramidal tiled TIFF.
    ///
    /// File extensions
    ///     .tif
    GenericTiledTiff,
}

/// The different ways the u8 color values are encoded into a u32 value.
///
/// A successfull reading from OpenSlide's `read_region()` will result in a buffer of `u32` with
/// `height * width` elements, where `height` and `width` is the shape (in pixels) of the read
/// region. This `u32` value consist of four `u8` values which are the red, green, blue, and alpha
/// value of a certain pixel. This enum determines in which order to arange these channels within
/// one element.
#[derive(Clone, Debug)]
pub enum WordRepresentation {
    /// From most significant bit to least significant bit: `[alpha, red, green, blue]`
    BigEndian,
    /// From most significant bit to least significant bit: `[blue, green, red, alpha]`
    LittleEndian,
}

/// This function takes a buffer, as the one obtained from openslide::read_region, and decodes into
/// an Rgba image buffer.
pub fn decode_buffer(
    buffer: &[u32],
    height: u32,
    width: u32,
    word_representation: WordRepresentation,
) -> Result<RgbaImage, Error> {
    let mut rgba_image = RgbaImage::new(width, height);

    for (col, row, pixel) in rgba_image.enumerate_pixels_mut() {
        let curr_pos = row * width + col;
        let value = buffer[curr_pos as usize];

        let mut buf = [0; 4];
        match word_representation {
            WordRepresentation::BigEndian => byteorder::BigEndian::write_u32(&mut buf, value),
            WordRepresentation::LittleEndian => byteorder::BigEndian::write_u32(&mut buf, value),
        };
        let [alpha, mut red, mut green, mut blue] = buf;

        if alpha != 0 && alpha != 255 {
            red = (red as f32 * (255.0 / alpha as f32))
                .round()
                .max(0.0)
                .min(255.0) as u8;
            green = (green as f32 * (255.0 / alpha as f32))
                .round()
                .max(0.0)
                .min(255.0) as u8;
            blue = (blue as f32 * (255.0 / alpha as f32))
                .round()
                .max(0.0)
                .min(255.0) as u8;
        }

        *pixel = Rgba([red, green, blue, alpha]);
    }

    Ok(rgba_image)
}

extern crate image;

mod openslide;

use image::DynamicImage;
use serde_json::json;
use std::error::Error;
use std::ops::Div;
use std::{path::Path, vec};

#[derive(PartialEq, Debug)]
struct TileInfo {
    l0_location: (u64, u64),
    slide_level: u64,
    l_size: (u64, u64),
    z_size: (u64, u64),
}

type Tile = DynamicImage;
pub struct DeepZoomGenerator {
    // We have four coordinate planes:
    // - Row and column of the tile within the Deep Zoom level (t_)
    // - Pixel coordinates within the Deep Zoom level (z_)
    // - Pixel coordinates within the slide level (l_)
    // - Pixel coordinates within slide level 0 (l0_)
    wsi: openslide::OpenSlide,
    l0_dimensions: (u64, u64),
    level_dimensions: Vec<(u64, u64)>,
    z_dimensions: Vec<(u64, u64)>,
    t_dimensions: Vec<(u64, u64)>,
    _slide_from_dz_level: Vec<u32>,
    _l_z_downsamples: Vec<f64>,
    _l0_offset: (u64, u64),
    l0_l_downsamples: Vec<f64>,
    tile_size: u64,
    overlap: u64,
}

impl DeepZoomGenerator {
    pub fn new(wsi_path: &Path) -> Result<DeepZoomGenerator, Box<dyn Error>> {
        // TODO: make these parameters configurable.
        let tile_size: u64 = 254;
        let overlap: u64 = 1;
        let _l0_offset: (u64, u64) = (0, 0);

        let wsi = openslide::OpenSlide::new(&wsi_path)?;
        let l0_dimensions = wsi.get_level0_dimensions()?; // (width, height)
        let level_count = wsi.get_level_count()?;
        let mut level_dimensions: Vec<(u64, u64)> = Vec::new();
        for lvl in 0..level_count {
            level_dimensions.push(wsi.get_level_dimensions(lvl)?)
        }

        let mut l0_l_downsamples: Vec<f64> = Vec::new();
        for lvl in 0..level_count {
            l0_l_downsamples.push(wsi.get_level_downsample(lvl)?);
        }

        // Derive all possible Deep Zoom levels.
        let mut z_size: (u64, u64) = l0_dimensions.clone();
        let mut z_dimensions: Vec<(u64, u64)> = vec![z_size.clone()];
        while z_size.0 > 1 || z_size.1 > 1 {
            let (w, h) = z_size;
            z_size = (
                ((w as f64).div(2.0).ceil() as u64).max(1),
                ((h as f64).div(2.0).ceil() as u64).max(1),
            );
            z_dimensions.push(z_size);
        }
        z_dimensions.reverse();

        let t_dimensions: Vec<(u64, u64)> = z_dimensions
            .iter()
            .map(|(zw, zh)| {
                (
                    (*zw as f64).div(tile_size as f64).ceil() as u64,
                    (*zh as f64).div(tile_size as f64).ceil() as u64,
                )
            })
            .collect();

        //  Deep Zoom level count
        let dz_levels = z_dimensions.len() as u64;

        // Total downsamples for each Deep Zoom level
        let l0_z_downsamples: Vec<u64> = (0..dz_levels)
            .map(|lvl| 2u64.pow((dz_levels - lvl - 1) as u32))
            .collect();

        // Preferred slide levels for each Deep Zoom level
        let mut _slide_from_dz_level: Vec<u32> = Vec::new();
        for lvl in &l0_z_downsamples {
            _slide_from_dz_level.push(wsi.get_best_level_for_downsample(*lvl)?);
        }

        // Piecewise downsamples
        let mut _l_z_downsamples: Vec<f64> = Vec::new();
        for dz_level in 0..dz_levels {
            // TODO: using array indexing; assert assumptions about array size
            let slide_level = _slide_from_dz_level[dz_level as usize];
            let ds = (l0_z_downsamples[dz_level as usize] as f64)
                .div(wsi.get_level_downsample(slide_level)?);
            _l_z_downsamples.push(ds);
        }

        return Ok(DeepZoomGenerator {
            wsi,
            l0_dimensions,
            level_dimensions,
            z_dimensions,
            t_dimensions,
            _slide_from_dz_level,
            _l_z_downsamples,
            _l0_offset,
            l0_l_downsamples,
            tile_size,
            overlap,
        });
    }

    pub fn get_dzi(&self) -> String {
        let (w, h) = self.l0_dimensions;
        let data = json!({
            "Image": {
                "xmlns":    "http://schemas.microsoft.com/deepzoom/2008",
                "Format":   "jpg",
                "Overlap":  self.overlap,
                "TileSize": self.tile_size,
                "Size": {
                    "Height": h,
                    "Width":  w,
                }
            }
        });
        return data.to_string();
    }

    fn get_tile_info(&self, dz_level: u64, t_location: (u64, u64)) -> Result<TileInfo, String> {
        if dz_level >= self.z_dimensions.len() as u64 {
            return Err(format!(
                "dz_level {} exceeds number of z-dimensions in slide ({})",
                dz_level,
                self.z_dimensions.len()
            ));
        }
        let t_lim = self.t_dimensions[dz_level as usize];
        if t_location.0 >= t_lim.0 || t_location.1 >= t_lim.1 {
            return Err(format!(
                "t_location {:?} exceeds t_lim of slide ({:?})",
                t_location, t_lim
            ));
        }

        // Get preferred slide level
        let slide_level = self._slide_from_dz_level[dz_level as usize] as u64;

        // Calculate top/left and bottom/right overlap
        let z_overlap_tl = (
            if t_location.0 != 0 {
                self.overlap * 1
            } else {
                0
            },
            if t_location.1 != 0 {
                self.overlap * 1
            } else {
                0
            },
        );
        let z_overlap_br = (
            if t_location.0 != (t_lim.0 - 1) {
                self.overlap * 1
            } else {
                0
            },
            if t_location.1 != (t_lim.1 - 1) {
                self.overlap * 1
            } else {
                0
            },
        );

        // Get final size of the tile
        let z_lim = self.z_dimensions[dz_level as usize];
        let z_size = (
            self.tile_size.min(z_lim.0 - self.tile_size * t_location.0)
                + z_overlap_tl.0
                + z_overlap_br.0,
            self.tile_size.min(z_lim.1 - self.tile_size * t_location.1)
                + z_overlap_tl.1
                + z_overlap_br.1,
        );

        // Obtain the region coordinates
        let z_location = (self.tile_size * t_location.0, self.tile_size * t_location.1);

        let lz = self._l_z_downsamples[dz_level as usize];
        let l_location = (
            lz * (z_location.0 as f64 - z_overlap_tl.0 as f64),
            lz * (z_location.1 as f64 - z_overlap_tl.1 as f64),
        );

        // Round location down and size up, and add offset of active area
        let l0_sl = self.l0_l_downsamples[slide_level as usize];
        let l0_location = (
            (l0_sl * l_location.0).round() as u64 + self._l0_offset.0,
            (l0_sl * l_location.1).round() as u64 + self._l0_offset.1,
        );

        let l_lim = self.level_dimensions[slide_level as usize];
        let l_size = (
            (lz * z_size.0 as f64)
                .ceil()
                .min(l_lim.0 as f64 - l_location.0.ceil())
                .round() as u64,
            (lz * z_size.1 as f64)
                .ceil()
                .min(l_lim.1 as f64 - l_location.1.ceil())
                .round() as u64,
        );

        // Return read_region() parameters plus tile size for final scaling
        return Ok(TileInfo {
            l0_location,
            slide_level,
            l_size,
            z_size,
        });
    }

    pub fn get_tile(&self, level: u64, col: u64, row: u64) -> Result<Tile, Box<dyn Error>> {
        let tile_info = self.get_tile_info(level, (col, row))?;

        // Note that the rust openslide bindings read_region expects (row,col) and not (col,row) like the C & python impl.
        let tile = self.wsi.read_region(
            tile_info.l0_location.1,
            tile_info.l0_location.0,
            tile_info.slide_level,
            // Note that the rust openslide bindings
            // read_region expects (height, width) and not (width,height) like the C & python impl.
            tile_info.l_size.1,
            tile_info.l_size.0,
        )?;

        // Scale the tile to the correct size
        let (desired_w, desired_h) = tile_info.z_size;
        let (w, h) = tile.dimensions();
        if (desired_w as u32) != w || (desired_h as u32) != h {
            // TODO: revist interpolation method used. May be able to speed this up?
            let resized = image::imageops::thumbnail(&tile, desired_w as u32, desired_h as u32);
            return Ok(DynamicImage::ImageRgba8(resized));
        }
        return Ok(DynamicImage::ImageRgba8(tile));
    }
}

#[test]
fn sanity_check_initialisation() {
    // TODO: example slide (~2GB) available upon request.
    let filename = Path::new("demodata/example.svs");
    let g = DeepZoomGenerator::new(filename).unwrap();

    // Below test cases assume these parameters:
    assert_eq!(g.tile_size, 254);
    assert_eq!(g.overlap, 1);

    assert_eq!(g.l0_dimensions, (113288, 83050));
    assert_eq!(
        g.level_dimensions,
        vec![(113288, 83050), (28322, 20762), (7080, 5190), (3540, 2595)]
    );
    assert_eq!(
        g.z_dimensions,
        vec![
            (1, 1),
            (2, 2),
            (4, 3),
            (7, 6),
            (14, 11),
            (28, 21),
            (56, 41),
            (111, 82),
            (222, 163),
            (443, 325),
            (886, 649),
            (1771, 1298),
            (3541, 2596),
            (7081, 5191),
            (14161, 10382),
            (28322, 20763),
            (56644, 41525),
            (113288, 83050)
        ]
    );
    assert_eq!(
        g.t_dimensions,
        vec![
            (1, 1),
            (1, 1),
            (1, 1),
            (1, 1),
            (1, 1),
            (1, 1),
            (1, 1),
            (1, 1),
            (1, 1),
            (2, 2),
            (4, 3),
            (7, 6),
            (14, 11),
            (28, 21),
            (56, 41),
            (112, 82),
            (224, 164),
            (447, 327)
        ]
    );
    assert_eq!(
        g._slide_from_dz_level,
        vec![3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 2, 1, 1, 0, 0, 0]
    );
    assert_eq!(
        g._l_z_downsamples,
        vec![
            4095.608776471337,
            2047.8043882356685,
            1023.9021941178343,
            511.9510970589171,
            255.97554852945856,
            127.98777426472928,
            63.99388713236464,
            31.99694356618232,
            15.99847178309116,
            7.99923589154558,
            3.99961794577279,
            1.999808972886395,
            1.999808972886395,
            3.9999518356632833,
            1.9999759178316416,
            4.0,
            2.0,
            1.0
        ]
    );
    assert_eq!(
        g.l0_l_downsamples,
        vec![
            1.0,
            4.000048164916675,
            16.001528362888216,
            32.00305672577643
        ]
    )
}

#[test]
fn test_get_tile_info() {
    let filename = Path::new("demodata/example.svs");
    let g = DeepZoomGenerator::new(filename).unwrap();
    assert_eq!(
        g.get_tile_info(13, (4, 4)).unwrap(),
        TileInfo {
            l0_location: (16240, 16240),
            slide_level: 1,
            l_size: (1024, 1024),
            z_size: (256, 256),
        }
    );
}

#[test]
fn test_get_tile_0() {
    use image::GenericImageView;
    let filename = Path::new("demodata/example.svs");
    let g = DeepZoomGenerator::new(filename).unwrap();
    let tile = g.get_tile(13, 4, 4).unwrap();
    assert_eq!(tile.dimensions(), (256, 256));
}

#[test]
fn test_get_tile_out_of_bounds() {
    let filename = Path::new("demodata/example.svs");
    let g = DeepZoomGenerator::new(filename).unwrap();
    let res = g.get_tile(12, 5, 11);
    match res {
        Ok(_val) => panic!("Should error out"),
        Err(_e) => {
            // TODO: use a more sensible error type and check the message contained within.
        }
    }
}

use std::{
    cell::RefCell,
    collections::HashMap,
    io::Cursor,
    sync::Arc,
};

use anyhow::{Context, Result};
use vello_common::pixmap::Pixmap;
use vello_cpu::peniko::color::PremulRgba8;

use crate::blend2d::generated::images;

pub struct Sprites {
    originals: [Arc<Pixmap>; 4],
    scaled: RefCell<HashMap<u32, [Arc<Pixmap>; 4]>>,
}

impl Sprites {
    pub fn load() -> Result<Self> {
        let babelfish = Arc::new(load_png(images::images::_RESOURCE_BABELFISH_PNG)?);
        let ksplash = Arc::new(load_png(images::images::_RESOURCE_KSPLASH_PNG)?);
        let ktip = Arc::new(load_png(images::images::_RESOURCE_KTIP_PNG)?);
        let firewall = Arc::new(load_png(images::images::_RESOURCE_FIREWALL_PNG)?);
        Ok(Self {
            originals: [babelfish, ksplash, ktip, firewall],
            scaled: RefCell::new(HashMap::new()),
        })
    }

    pub fn sprite(&self, index: usize, size: u32) -> Arc<Pixmap> {
        if size == 0 {
            return self.originals[index].clone();
        }
        if let Some(entry) = self.scaled.borrow().get(&size) {
            return entry[index].clone();
        }
        let resized = [
            Arc::new(scale_pixmap(&self.originals[0], size)),
            Arc::new(scale_pixmap(&self.originals[1], size)),
            Arc::new(scale_pixmap(&self.originals[2], size)),
            Arc::new(scale_pixmap(&self.originals[3], size)),
        ];
        let result = resized[index].clone();
        self.scaled.borrow_mut().insert(size, resized);
        result
    }
}

fn load_png(bytes: &[u8]) -> Result<Pixmap> {
    Pixmap::from_png(Cursor::new(bytes)).context("failed to decode sprite png")
}

fn scale_pixmap(src: &Pixmap, size: u32) -> Pixmap {
    let dst_size = size as u16;
    let mut dst = Pixmap::new(dst_size, dst_size);
    let src_w = src.width() as u32;
    let src_h = src.height() as u32;
    if src_w == size && src_h == size {
        dst.data_mut().copy_from_slice(src.data());
        return dst;
    }

    let src_pixels = src.data();
    let dst_pixels = dst.data_mut();
    for y in 0..size {
        let sy = y.saturating_mul(src_h) / size;
        for x in 0..size {
            let sx = x.saturating_mul(src_w) / size;
            let dst_idx = (y * size + x) as usize;
            let src_idx = (sy * src_w + sx) as usize;
            dst_pixels[dst_idx] = src_pixels[src_idx];
        }
    }
    dst
}

pub fn copy_pixmap(src: &Pixmap) -> Pixmap {
    let mut dst = Pixmap::new(src.width(), src.height());
    dst.data_mut().copy_from_slice(src.data());
    dst
}

pub fn blit(src: &Pixmap, dst: &mut Pixmap, origin_x: i32, origin_y: i32) {
    let sw = src.width() as i32;
    let sh = src.height() as i32;
    let dw = dst.width() as i32;
    let dh = dst.height() as i32;
    for y in 0..sh {
        let dy = origin_y + y;
        if dy < 0 || dy >= dh {
            continue;
        }
        let src_row = &src.data()[(y as usize) * (sw as usize)..][..sw as usize];
        let dst_row_start = (dy as usize) * (dw as usize);
        for x in 0..sw {
            let dx = origin_x + x;
            if dx < 0 || dx >= dw {
                continue;
            }
            dst.data_mut()[dst_row_start + dx as usize] = src_row[x as usize];
        }
    }
}

pub fn clear_pixmap(pixmap: &mut Pixmap, color: PremulRgba8) {
    for pixel in pixmap.data_mut() {
        *pixel = color;
    }
}

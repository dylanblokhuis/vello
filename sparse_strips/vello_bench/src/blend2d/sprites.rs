use vello_common::pixmap::Pixmap;
use vello_cpu::peniko::color::PremulRgba8;

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

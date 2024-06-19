// use std::path::Path;
//
// use bevy_math::IVec2;
// use egui_macroquad::macroquad::{color::Color, texture::Image};
//
// /// Saves image without flipping it
// pub trait ImageExtensions {
//     fn save_png(&self, path: impl AsRef<Path>);
//     fn empty_with_size(size: IVec2) -> Image;
//     fn draw_image(&mut self, other: &Image, pos: IVec2);
//     fn set_pixel_checked(&mut self, pos: IVec2);
//     fn get_pixel_checked(&self, pos: IVec2) -> Option<Color>;
//     fn get_index(&self, pos: IVec2) -> Option<usize>;
// }
//
// impl ImageExtensions for Image {
//     fn save_png(&self, path: impl AsRef<Path>) {
//         image::save_buffer(
//             path,
//             &self.bytes[..],
//             self.width as _,
//             self.height as _,
//             image::ColorType::Rgba8,
//         )
//         .unwrap();
//     }
//
//     fn empty_with_size(size: IVec2) -> Image {
//         let mut empty = Image::empty();
//         empty.width = size.x as u16;
//         empty.height = size.y as u16;
//         empty.bytes = vec![0; (size.x * size.y * 4) as usize];
//         empty
//     }
//
//     fn draw_image(&mut self, other: &Image, pos: IVec2) {
//         todo!()
//     }
//
//     fn set_pixel_checked(&mut self, pos: IVec2) {
//
//     }
//
//     fn get_pixel_checked(&self, pos: IVec2) -> Option<Color> {
//         todo!()
//     }
// }

use core::fmt::Arguments; 

use alloc::fmt::format;
use ab_glyph::{point, FontRef, Font, ScaleFont, PxScaleFont};

use crate::term::framebuffer::EFIFrameBuffer;

use ansi_parser::{Output, AnsiParser};

pub use super::framebuffer_color::FBColor;

#[derive(Debug)]
pub struct FBTerm<'a> {
    framebuffer: Option<EFIFrameBuffer<'a>>,
    term_font: PxScaleFont<FontRef<'a>>,
    current_column: usize,
    current_row: usize,

    character_height: usize,
    character_width: usize,

    max_row: usize,
    max_column: usize,

    foreground_color: FBColor,
    background_color: FBColor,
}


impl<'a> FBTerm<'a> {
    pub fn write_fmt(&mut self, args: Arguments) {
        self.print(format(args));
    }

    pub fn set_background(&mut self, color: FBColor) {
        self.background_color = color;
    }

    pub fn clear(&mut self) {
        if let Some(fb) = &mut self.framebuffer {
            let w = fb.get_width();
            let h = fb.get_height();

            for i in 0..w*h {
                fb.pixels[i] = self.background_color.clone().into();
            }
        }
    }

    pub fn new_unset(font: FontRef<'a>) -> Self  {
        let scaled_font = font.into_scaled(25.0);

        let tc = scaled_font.glyph_id(' ');
        let character_width = scaled_font.h_advance(tc) as usize;
        let character_height  = scaled_font.height() as usize;

        FBTerm {
            framebuffer: None,
            term_font: scaled_font,
            current_column: 0,
            current_row: 0,

            character_height,
            character_width,

            max_column: 0,
            max_row: 0,

            foreground_color: FBColor::Pink,
            background_color: FBColor::Rgb(0,0,0),
        }
    }

    pub fn set_framebuffer(&mut self, framebuffer: EFIFrameBuffer<'a>) {
        let width = framebuffer.get_width();
        let height = framebuffer.get_height();

        self.framebuffer = Some(framebuffer);

        self.max_column = (width - (width % self.character_width)) / self.character_width;
        self.max_row = (height - (height % self.character_height))  / self.character_height;
    }

    pub fn print_char_at(&mut self, x: usize, y: usize, c: char) {
        let glyph = self.term_font.glyph_id(c).with_scale_and_position(25.0, point(0.0, self.term_font.ascent()));
        if let Some(fb) = &mut self.framebuffer {
            if let Some(glyph) = self.term_font.outline_glyph(glyph) {
                let bounds = glyph.px_bounds();

                let x_offset: usize = (bounds.min.x + (x * self.character_width) as f32) as usize;
                let y_offset: usize = (bounds.min.y + (y * self.character_height) as f32) as usize;

                glyph.draw(|x,y,v| { 
                    fb.draw_pixel(x as usize + x_offset,  y as usize + y_offset, 
                                  if v >= 0.5 {
                                      self.foreground_color.clone()
                                  } else {
                                      self.background_color.clone()
                                  })
                });
            }
        }
    }

    pub fn print_ref_str(&mut self, s: &str) {
        for block in s.ansi_parse() {
            match block {
                Output::TextBlock(text) => {
                    for c in text.chars() {
                        if !c.is_control() {
                            self.print_char_at(self.current_column, self.current_row, c);
                        }
                        self.update_location(c);
                    }
                }
                Output::Escape(_) => {}
            }
        }
    }

    pub fn print(&mut self, s: impl AsRef<str>) {
        self.print_ref_str(s.as_ref());
    }

    pub fn scroll_screen(&mut self) {
        if let Some(fb) = &mut self.framebuffer {
            let end_pixel = self.character_height * fb.get_width() * (self.max_row - 1);
            let line_offset = self.character_height * fb.get_width();

            fb.shift_left(line_offset);

            for pixel in end_pixel..end_pixel+line_offset {
                fb.pixels[pixel] = self.background_color.clone().into();
            }
            self.current_row -= 1;
        }
    }

    fn update_location(&mut self, character: char) {
        match character {
            '\n' => {
                self.current_row += 1;
                self.current_column = 0;
            },
            _ => { 
                self.current_column += 1;
            }
        } 

        if self.current_column >= self.max_column {
            self.current_row += 1;
            self.current_column = 0;
        }

        if self.current_row >= self.max_row {
            self.scroll_screen();
        }

    }
}

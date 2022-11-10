use png;

#[derive(Debug)]
pub struct Frame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub x_offset: u32,
    pub y_offset: u32,
    pub delay_num: u16,
    pub delay_den: u16,
    pub dispose_op: png::DisposeOp,
    pub blend_op: png::BlendOp,
    pub pixels: Option<Vec<u8>>,
}

impl Frame {
    pub fn new(
        data: Vec<u8>,
        width: u32,
        height: u32,
        x_offset: u32,
        y_offset: u32,
        delay_num: u16,
        delay_den: u16,
        dispose_op: png::DisposeOp,
        blend_op: png::BlendOp,
    ) -> Frame {
        Frame {
            data,
            width,
            height,
            x_offset,
            y_offset,
            delay_num,
            delay_den,
            dispose_op,
            blend_op,
            pixels: None,
        }
    }
}

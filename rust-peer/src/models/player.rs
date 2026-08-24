use fomoxa_attributes::*;

#[network]
#[codec(edge)]
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Player {
    #[network(u32)]
    #[codec(edge)]
    pub id: u32,

    #[network(f32)]
    #[codec(edge)]
    pub x: f32,

    #[network(f32)]
    #[codec(edge)]
    pub y: f32,
}

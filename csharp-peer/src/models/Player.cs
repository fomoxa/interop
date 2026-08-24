using Fomoxa;

namespace Models;

[Network]
[Codec("edge")]
public class Player
{
    [Network("u32")]
    [Codec("edge")]
    public uint Id { get; set; }

    [Network("f32")]
    [Codec("edge")]
    public float X { get; set; }

    [Network("f32")]
    [Codec("edge")]
    public float Y { get; set; }
}

package models

//fomoxa:model codec=edge
type Player struct {
	ID uint32  `fomoxa:"u32" codec:"edge"`
	X  float32 `fomoxa:"f32" codec:"edge"`
	Y  float32 `fomoxa:"f32" codec:"edge"`
}

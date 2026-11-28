package p

var x int // @Router /x [get]
func Below() {}

func Inside() {
	// @Router /y [get]
	_ = "// @Router /s [get]"
}

// @Router /z [get]
var NotFunc int

// @Router /w [get]
func Attached() {}

// A directive below its function belongs to nothing.
func Before() {}

// @Router /v [get]

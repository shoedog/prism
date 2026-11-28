package p

// @Router x [get]
func Relative() {}

// @Router https://h/x [get]
func Absolute() {}

// @Router /x?q=1 [get]
func Query() {}

// @Router /x#frag [get]
func Fragment() {}

// @Router /x/%7Bid%7D [get]
func Encoded() {}

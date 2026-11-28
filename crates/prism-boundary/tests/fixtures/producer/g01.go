package p

type T struct{}

// H handles x.
// @Summary x
// @Router /x [get]
func (*T) H() {}

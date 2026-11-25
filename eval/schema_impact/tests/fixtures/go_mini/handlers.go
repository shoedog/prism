package example

type Controller[T any] struct{}

// Handler serves one operation.
// @Router /api/x [get]
func (c *Controller[T]) Handler() {}

// List serves two operations.
// @Router /api/a [GET]
//
// @Router /api/b/{name} [post]
func List() {}

// @Router /detached [get]

func Detached() {}

// @Router /outside [head]
func Outside() {
	// @Router /inside [get]
	_ = func() {}
}

/*
// @Router /block [get]
*/
func Block() {}

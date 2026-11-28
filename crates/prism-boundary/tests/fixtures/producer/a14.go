package p

type T[A any] struct{}

// @Router /x [get]
func (t *T[A]) Ptr() {}

// @Router /y [get]
func (T[A]) Value() {}

// @Router /z [get]
func Bodyless()

package fixture

func new[T any](x T) T { return x }
func make[T any](x T) T { return x }

func shadowed() {
	_ = new[int](3)
	_ = make[[]int]([]int{})
}

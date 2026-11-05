package pkg

type A struct{ Run func() }
type B struct{}

func (b *B) Run() {}

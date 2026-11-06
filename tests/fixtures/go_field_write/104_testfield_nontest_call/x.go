package pkg

func use(n *N, v int) {
	n.run.f = v
	n.run.init()
}

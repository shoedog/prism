package example

import "net/http"

func routes() {
	g.GET("/x", c.Handler)
	g.GET(
		`/x/:id`, // Literal suffix, split across lines.
		h,
	)
	g.GET("\x2f" + "computed", h)
	g.HEAD("\x2foutside", c.Nested.Outside)
	g.Any("/y", h)
	g.Handle(verb, prefix, h)
	g.Match(methods, prefix, h)
	// g.GET("/ignored", c.Handler)
	_ = `g.GET("/also-ignored", c.Handler)`
	http.Get("https://example.invalid")
}

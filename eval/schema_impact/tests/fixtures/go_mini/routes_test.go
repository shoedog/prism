package example

// @Router /test-only [get]
func TestRoutes() {
	g.GET("/test-only", h)
	g.Any("/test-only", h)
	http.Get("https://example.invalid")
}

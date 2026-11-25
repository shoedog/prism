package routes

import "github.com/gin-gonic/gin"

func release(c *gin.Context) { c.Status(204) }

func NewRouter(method string) *gin.Engine {
	app := gin.New()
	app.Handle(method, "/release", release)
	return app
}

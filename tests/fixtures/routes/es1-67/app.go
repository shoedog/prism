package routes

import "github.com/gin-gonic/gin"

func status(c *gin.Context) { c.Status(204) }

func NewRouter() *gin.Engine {
	app := gin.New()
	app.Match([]string{"HEAD", "GET"}, "/status", status)
	return app
}

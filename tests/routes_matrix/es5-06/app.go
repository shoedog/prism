package routes

import "github.com/gin-gonic/gin"

func check(c *gin.Context) { c.Next() }
func show(c *gin.Context)  { c.Status(204) }

func NewApp() *gin.Engine {
	app := gin.New()
	chain := []gin.HandlerFunc{check, show}
	app.GET("/chain", chain...)
	app.GET("/direct", check, show)
	return app
}

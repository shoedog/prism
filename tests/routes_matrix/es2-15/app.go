package routes

import "github.com/gin-gonic/gin"

func exports(c *gin.Context) { c.Status(204) }

func NewRouter() *gin.Engine {
	app := gin.New()
	team := app.Group("/team//")
	team.GET("./exports/", exports)
	return app
}

package routes

import "github.com/gin-gonic/gin"

func audit(c *gin.Context) { c.Next() }
func gate(c *gin.Context)  { c.Next() }
func trace(c *gin.Context) { c.Next() }
func stats(c *gin.Context) { c.Status(200) }

func NewRouter() *gin.Engine {
	app := gin.New()
	app.Use(audit)
	staff := app.Group("/staff", gate)
	staff.GET("/before", stats)
	staff.Use(trace)
	staff.GET("/after", stats)
	return app
}

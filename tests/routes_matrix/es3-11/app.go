package routes

import "github.com/gin-gonic/gin"

func status(c *gin.Context) { c.String(200, "ok") }

func App() *gin.Engine {
	r := gin.New()
	r.Group("/ops").Group("/v2").GET("/status", status)
	return r
}

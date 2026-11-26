package routes

import "github.com/gin-gonic/gin"

func status(c *gin.Context)      { c.String(200, "ok") }
func unsupported(c *gin.Context) { c.String(405, "unsupported") }

func App() *gin.Engine {
	r := gin.New()
	r.HandleMethodNotAllowed = true
	r.NoMethod(unsupported)
	r.GET("/status", status)
	return r
}

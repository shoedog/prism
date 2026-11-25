package routes

import "github.com/gin-gonic/gin"

func status(c *gin.Context) {
	c.String(200, "ready")
}

func NewApp() *gin.Engine {
	engine := gin.New()
	engine.RouterGroup.GET("/status", status)
	return engine
}

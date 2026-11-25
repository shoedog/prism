package fixture

import "github.com/gin-gonic/gin"

func list(c *gin.Context) {}

func NewApp() *gin.Engine {
	r := gin.New()
	handler := list
	r.GET("/orders", handler)
	return r
}

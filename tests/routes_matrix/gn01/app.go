package fixture

import "github.com/gin-gonic/gin"

func list(c *gin.Context) {}

func NewApp() *gin.Engine {
	r := gin.Default()
	r.GET("/x", list)
	return r
}

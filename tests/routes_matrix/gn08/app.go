package fixture

import "github.com/gin-gonic/gin"

func auth() gin.HandlerFunc {
	return func(c *gin.Context) {
		c.Next()
	}
}

func h(c *gin.Context) {}

func NewApp() *gin.Engine {
	r := gin.New()
	r.GET("/x", auth(), h)
	return r
}

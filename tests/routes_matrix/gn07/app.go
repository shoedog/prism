package fixture

import "github.com/gin-gonic/gin"

func h(c *gin.Context) {}

func NewApp() *gin.Engine {
	r := gin.New()
	r.Handle("GET", "/x", h)
	r.Any("/status", h)
	return r
}

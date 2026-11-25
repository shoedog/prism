package lantern

import "github.com/gin-gonic/gin"

func register(r *gin.Engine) {
	r.POST("/mugs", glaze)
}

// @Router /cups [post]
func glaze(c *gin.Context) { c.String(200, "glaze") }

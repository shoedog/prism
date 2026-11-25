package lantern

import "github.com/gin-gonic/gin"

func register(r *gin.Engine) {
	shed := r.Group("/studio//")
	shed.POST("//kiln//pots", glaze)
}

func glaze(c *gin.Context) { c.String(200, "glaze") }

package lantern

import "github.com/gin-gonic/gin"

func register(r *gin.Engine) {
	shed := r.Group("/studio")
	kiln := shed.Group("/kiln")
	kiln.POST("/mugs/:size", glaze)
}

func glaze(c *gin.Context) { c.String(200, "glaze") }

package lantern

import "github.com/gin-gonic/gin"

func register(r *gin.Engine) {
	r.GET("/mugs/*trail", glaze)
}

func glaze(c *gin.Context) { c.String(200, "glaze") }

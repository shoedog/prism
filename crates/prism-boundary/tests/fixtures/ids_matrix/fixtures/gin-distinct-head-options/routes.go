package lantern

import "github.com/gin-gonic/gin"

func register(r *gin.Engine) {
	r.HEAD("/mugs", glaze)
	r.OPTIONS("/mugs", glaze)
}

func glaze(c *gin.Context) { c.String(200, "glaze") }

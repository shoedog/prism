package lantern

import "github.com/gin-gonic/gin"

func register(r *gin.Engine) {
	r.Handle("PROPFIND", "/mugs", glaze)
}

func glaze(c *gin.Context) { c.String(200, "glaze") }

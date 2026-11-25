package lantern

import "github.com/gin-gonic/gin"

func register(r *gin.Engine) {
	install(r.Group("/studio"))
}

func glaze(c *gin.Context) { c.String(200, "glaze") }

func install(group *gin.RouterGroup) {
	group.POST("/mugs", glaze)
}

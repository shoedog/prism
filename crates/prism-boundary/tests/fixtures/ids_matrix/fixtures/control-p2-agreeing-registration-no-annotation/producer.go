package lantern

import "github.com/gin-gonic/gin"

func register(r *gin.Engine) { r.GET("/mugs", Glaze) }

func Glaze(c *gin.Context) {}

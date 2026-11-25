package lantern

import "github.com/gin-gonic/gin"

func register(r *gin.Engine) { r.POST("/mugs", Glaze) }

// @Router /mugs [get]
func Glaze(c *gin.Context) {}

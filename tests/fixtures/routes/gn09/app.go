package fixture

import "github.com/gin-gonic/gin"

func register(r *gin.RouterGroup) {
	r.GET("/health", health)
}

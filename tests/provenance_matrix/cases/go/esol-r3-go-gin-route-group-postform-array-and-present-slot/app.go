package fixture

import "github.com/gin-gonic/gin"

func router() *gin.Engine {
  app := gin.New()
  catalog := app.Group("/catalog")
  catalog.Handle("POST", "/preview", func(incoming *gin.Context) {
    labels, present := incoming.GetPostFormArray("label")
    _, _ = labels, present
  })
  return app
}

package main

import "github.com/gin-gonic/gin"

func handler(c *gin.Context) {
	name := c.Param("file")
	c.File(name)
}

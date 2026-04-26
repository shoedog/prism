package main

import (
	"os"
	"path/filepath"
	"strings"

	"github.com/gin-gonic/gin"
)

func handler(c *gin.Context) {
	name := c.Query("file")
	base := "/var/uploads"
	cleaned := filepath.Clean(filepath.Join(base, name))
	if !strings.HasPrefix(cleaned, base) {
		c.AbortWithStatus(403)
		return
	}
	_, _ = os.ReadFile(cleaned)
}

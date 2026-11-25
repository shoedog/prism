package catalog
import src "os"
func deployed() string {
    label := src.Getenv("EDITION")
    return label
}

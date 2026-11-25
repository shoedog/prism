package catalog
import src "example.org/catalog/fixed"
func preview() string {
    label := src.Getenv("EDITION")
    return label
}

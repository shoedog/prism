variable "regions" {
  type = list(string)
}
output "primary" {
  value = var.regions[0]
}

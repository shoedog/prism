variable "label" { type = string }

output "labels" {
  value = [for var in [{ label = "fixed" }] : var.label]
}

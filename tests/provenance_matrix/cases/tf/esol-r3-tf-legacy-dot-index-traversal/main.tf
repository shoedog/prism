variable "labels" { type = list(string) }

output "primary" {
  value = var.labels.0
}

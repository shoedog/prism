variable "nodes" {
  type = list(object({ label = string }))
}

output "labels" {
  value = var.nodes[*].label
}

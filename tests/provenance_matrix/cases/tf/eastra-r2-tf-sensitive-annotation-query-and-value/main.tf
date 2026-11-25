variable "edition" { type = string }
locals {
  annotated = sensitive(var.edition)
  label     = nonsensitive(local.annotated)
  marked    = issensitive(local.annotated)
}
output "snapshot" {
  value = {
    data = [for label in [local.label] :
      label
    ]
    flags = [for marked in [local.marked] :
      marked
    ]
  }
}

variable "labels" { type = map(string) }

resource "terraform_data" "by_label" {
  for_each = var.labels
  input    = each.key
}

resource "terraform_data" "by_number" {
  count = length(var.labels)
  input = count.index
}

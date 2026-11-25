function labels() {
  const fields = new URLSearchParams(location.search);
  fields.delete('preview');
  const edition = fields.get('edition');
  const preview = fields.get('preview') ?? 'local';
  return [edition, preview];
}

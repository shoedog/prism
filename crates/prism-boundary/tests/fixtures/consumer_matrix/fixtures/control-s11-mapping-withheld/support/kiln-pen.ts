export function inkAlderfenQuery(facets: Record<string, string>) {
  return new URLSearchParams(facets).toString();
}
